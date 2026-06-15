//! Состояние и чистая логика карусели миниатюр. Декод на rayon — в app.rs.

use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThumbState {
    Loading,
    Ready,
    Failed,
}

pub struct ThumbnailStore {
    states: HashMap<usize, ThumbState>,
    lru: VecDeque<usize>, // порядок использования (фронт — давний)
    limit: usize,
    /// Инкрементится при смене папки; результаты старого поколения игнорируются.
    pub generation: u64,
}

impl ThumbnailStore {
    pub fn new(limit: usize) -> Self {
        Self { states: HashMap::new(), lru: VecDeque::new(), limit, generation: 0 }
    }

    /// Сменить папку: сброс состояний, новое поколение.
    pub fn reset(&mut self) {
        self.states.clear();
        self.lru.clear();
        self.generation += 1;
    }

    pub fn state(&self, index: usize) -> Option<ThumbState> {
        self.states.get(&index).copied()
    }

    /// Индексы из `window`, которые ещё не запрашивались (нет состояния).
    /// Помечает их `Loading`. Возвращает список для декода.
    pub fn take_pending(&mut self, window: &[usize]) -> Vec<usize> {
        let mut pending = Vec::new();
        for &i in window {
            if !self.states.contains_key(&i) {
                self.states.insert(i, ThumbState::Loading);
                pending.push(i);
            }
        }
        pending
    }

    /// Отметить результат декода. Возвращает индексы, вытесненные по LRU
    /// (их текстуры надо освободить в ThumbnailLayer).
    pub fn mark_ready(&mut self, index: usize, ok: bool) -> Vec<usize> {
        self.states.insert(index, if ok { ThumbState::Ready } else { ThumbState::Failed });
        if ok {
            self.touch(index);
        }
        self.evict()
    }

    /// Обновить позицию индекса в LRU (использован/показан).
    pub fn touch(&mut self, index: usize) {
        if let Some(pos) = self.lru.iter().position(|&x| x == index) {
            self.lru.remove(pos);
        }
        self.lru.push_back(index);
    }

    /// Вытеснить готовые миниатюры сверх лимита. Возвращает освобождённые индексы.
    fn evict(&mut self) -> Vec<usize> {
        let mut freed = Vec::new();
        while self.lru.len() > self.limit {
            if let Some(old) = self.lru.pop_front() {
                self.states.remove(&old);
                freed.push(old);
            } else {
                break;
            }
        }
        freed
    }
}

/// Прямоугольник-источник для «cover»-кропа изображения src_w×src_h
/// под целевой аспект dst_w×dst_h. Возвращает (x, y, w, h) в пикселях источника.
pub fn cover_crop(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (u32, u32, u32, u32) {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return (0, 0, src_w, src_h);
    }
    let src_ar = src_w as f64 / src_h as f64;
    let dst_ar = dst_w as f64 / dst_h as f64;
    if src_ar > dst_ar {
        // источник шире — режем по бокам
        let w = (src_h as f64 * dst_ar).round() as u32;
        let x = (src_w - w) / 2;
        (x, 0, w.max(1), src_h)
    } else {
        // источник выше — режем сверху/снизу
        let h = (src_w as f64 / dst_ar).round() as u32;
        let y = (src_h - h) / 2;
        (0, y, src_w, h.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_pending_marks_loading_once() {
        let mut s = ThumbnailStore::new(256);
        let p = s.take_pending(&[0, 1, 2]);
        assert_eq!(p, vec![0, 1, 2]);
        // повторно — пусто (уже Loading)
        assert!(s.take_pending(&[0, 1, 2]).is_empty());
        assert_eq!(s.state(1), Some(ThumbState::Loading));
    }

    #[test]
    fn mark_ready_sets_state() {
        let mut s = ThumbnailStore::new(256);
        s.take_pending(&[5]);
        let freed = s.mark_ready(5, true);
        assert!(freed.is_empty());
        assert_eq!(s.state(5), Some(ThumbState::Ready));
    }

    #[test]
    fn mark_failed_sets_failed() {
        let mut s = ThumbnailStore::new(256);
        s.take_pending(&[5]);
        s.mark_ready(5, false);
        assert_eq!(s.state(5), Some(ThumbState::Failed));
    }

    #[test]
    fn lru_evicts_oldest_over_limit() {
        let mut s = ThumbnailStore::new(2);
        for i in 0..3 {
            s.take_pending(&[i]);
            s.mark_ready(i, true);
        }
        // лимит 2 → индекс 0 вытеснен
        let freed = s.mark_ready(2, true); // уже отмечен, но проверим состояние
        let _ = freed;
        assert_eq!(s.state(0), None);
        assert_eq!(s.state(1), Some(ThumbState::Ready));
        assert_eq!(s.state(2), Some(ThumbState::Ready));
    }

    #[test]
    fn reset_bumps_generation_and_clears() {
        let mut s = ThumbnailStore::new(256);
        s.take_pending(&[0]);
        let g = s.generation;
        s.reset();
        assert_eq!(s.generation, g + 1);
        assert_eq!(s.state(0), None);
    }

    #[test]
    fn cover_crop_wide_source() {
        // 200×100 в 64×64 → режем по бокам до 100×100
        let (x, y, w, h) = cover_crop(200, 100, 64, 64);
        assert_eq!((y, w, h), (0, 100, 100));
        assert_eq!(x, 50);
    }

    #[test]
    fn cover_crop_tall_source() {
        // 100×200 в 64×64 → режем сверху/снизу до 100×100
        let (x, y, w, h) = cover_crop(100, 200, 64, 64);
        assert_eq!((x, w, h), (0, 100, 100));
        assert_eq!(y, 50);
    }
}
