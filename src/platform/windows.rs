//! Нативный frameless: убираем caption через WM_NCCALCSIZE, но сохраняем
//! WS_THICKFRAME (resize/Aero Snap/тень). Hit-тест caption/краёв — WM_NCHITTEST.

use crate::ui::hit::{Edge, Region};
use crate::ui::{hit, layout};
use glam::Vec2;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, GetWindowLongPtrW, SetWindowPos, GWL_STYLE, HTBOTTOM,
    HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT,
    HTTOPRIGHT, NCCALCSIZE_PARAMS, SM_CXFRAME, SM_CXPADDEDBORDER, SM_CYFRAME, SWP_FRAMECHANGED,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_NCCALCSIZE, WM_NCHITTEST, WS_MAXIMIZE,
};

/// scale_factor × 100, общий для wndproc (нет доступа к winit-состоянию из FFI).
static SCALE_X100: AtomicU32 = AtomicU32::new(100);

/// Сообщить wndproc актуальный scale_factor.
pub fn set_scale(scale: f32) {
    SCALE_X100.store((scale * 100.0) as u32, Ordering::Relaxed);
}

fn scale() -> f32 {
    SCALE_X100.load(Ordering::Relaxed) as f32 / 100.0
}

/// Полноэкранный режим: в нём окно не двигают/не тянут — hit-test возвращает HTCLIENT.
static FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// Сообщить wndproc состояние fullscreen.
pub fn set_fullscreen(on: bool) {
    FULLSCREEN.store(on, Ordering::Relaxed);
}

/// Включить нативный frameless для окна winit. Безопасно при неудаче (логирует).
/// `hwnd_isize` — HWND как isize (из raw-window-handle).
pub fn enable(hwnd_isize: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_isize as *mut core::ffi::c_void);
    unsafe {
        // 1px-расширение DWM-фрейма в клиентскую область → системная тень.
        let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // Субклассируем wndproc (цепочка через DefSubclassProc).
        if !SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0).as_bool() {
            return Err("SetWindowSubclass провалился".into());
        }

        // Перерисовать рамку с новыми правилами NCCALCSIZE.
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        )
        .map_err(|e| format!("SetWindowPos: {e}"))?;
    }
    Ok(())
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_NCCALCSIZE if wparam.0 != 0 => {
            // Расширяем клиентскую область на весь caption.
            // В maximized-режиме компенсируем невидимые рамки, иначе контент срежется.
            let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
            // В fullscreen клиент = всё окно (монитор), без рамочного инсета —
            // иначе maximized-инсет срезает края и сквозь них виден рабочий стол.
            if FULLSCREEN.load(Ordering::Relaxed) {
                let _ = params;
                return LRESULT(0);
            }
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            if style & WS_MAXIMIZE.0 != 0 {
                // Системные метрики рамки maximized-окна.
                let fx = GetSystemMetrics(SM_CXFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                let fy = GetSystemMetrics(SM_CYFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER);
                params.rgrc[0].left += fx;
                params.rgrc[0].right -= fx;
                params.rgrc[0].top += fy;
                params.rgrc[0].bottom -= fy;
            }
            // Иначе оставляем rgrc[0] как есть → caption убран, клиент = всё окно.
            LRESULT(0)
        }
        WM_NCHITTEST => {
            if FULLSCREEN.load(Ordering::Relaxed) {
                return LRESULT(HTCLIENT as isize);
            }
            // Координаты курсора (экранные) → клиентские.
            let x = (lparam.0 & 0xffff) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
            let mut pt = windows::Win32::Foundation::POINT { x, y };
            let _ = ScreenToClient(hwnd, &mut pt);

            // Размер клиентской области.
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            let win = Vec2::new((rc.right - rc.left) as f32, (rc.bottom - rc.top) as f32);
            let cursor = Vec2::new(pt.x as f32, pt.y as f32);

            let s = scale();
            let l = layout::compute(win, s, 1.0, false);
            match hit::hit(&l, win, cursor, s) {
                Region::Caption => LRESULT(HTCAPTION as isize),
                Region::Resize(edge) => LRESULT(match edge {
                    Edge::Left => HTLEFT,
                    Edge::Right => HTRIGHT,
                    Edge::Top => HTTOP,
                    Edge::Bottom => HTBOTTOM,
                    Edge::TopLeft => HTTOPLEFT,
                    Edge::TopRight => HTTOPRIGHT,
                    Edge::BottomLeft => HTBOTTOMLEFT,
                    Edge::BottomRight => HTBOTTOMRIGHT,
                } as isize),
                // Кнопки и пустые места titlebar обрабатываем как клиент
                // (клики ловит winit; кнопки окна — в app.rs).
                _ => LRESULT(HTCLIENT as isize),
            }
        }
        _ => DefSubclassProc(hwnd, msg, wparam, lparam),
    }
}
