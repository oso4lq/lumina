//! Чистое ядро однострочного текстового редактора: буфер, каретка, выделение.
//! Без GPU/winit. Индексы — в символах (char), а не байтах (корректный Unicode).

#[derive(Debug, Clone, Default)]
pub struct TextEdit {
    chars: Vec<char>,
    caret: usize,          // позиция каретки в символах [0..=len]
    anchor: Option<usize>, // якорь выделения; None — нет выделения
}

impl TextEdit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_str(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let caret = chars.len();
        Self { chars, caret, anchor: None }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }
    pub fn caret(&self) -> usize {
        self.caret
    }
    pub fn len(&self) -> usize {
        self.chars.len()
    }
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn set_text(&mut self, s: &str) {
        self.chars = s.chars().collect();
        self.caret = self.chars.len();
        self.anchor = None;
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.caret = 0;
        self.anchor = None;
    }

    /// Границы выделения (lo, hi) в символах, если оно непустое.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor
            .map(|a| if a <= self.caret { (a, self.caret) } else { (self.caret, a) })
            .filter(|(a, b)| a != b)
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection().map(|(a, b)| self.chars[a..b].iter().collect())
    }

    /// Удалить выделение, если есть. Возвращает true, если что-то удалено.
    fn delete_selection(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            self.chars.drain(a..b);
            self.caret = a;
            self.anchor = None;
            true
        } else {
            self.anchor = None;
            false
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        for c in s.chars() {
            self.chars.insert(self.caret, c);
            self.caret += 1;
        }
        self.anchor = None;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret > 0 {
            self.caret -= 1;
            self.chars.remove(self.caret);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret < self.chars.len() {
            self.chars.remove(self.caret);
        }
    }

    fn set_caret(&mut self, pos: usize, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = pos.min(self.chars.len());
    }

    pub fn move_left(&mut self, extend: bool) {
        let pos = self.caret.saturating_sub(1);
        self.set_caret(pos, extend);
    }
    pub fn move_right(&mut self, extend: bool) {
        let pos = self.caret + 1;
        self.set_caret(pos, extend);
    }
    pub fn move_home(&mut self, extend: bool) {
        self.set_caret(0, extend);
    }
    pub fn move_end(&mut self, extend: bool) {
        let e = self.chars.len();
        self.set_caret(e, extend);
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.caret = self.chars.len();
    }

    /// Вырезать выделение: вернуть его текст и удалить из буфера.
    pub fn cut(&mut self) -> Option<String> {
        let s = self.selected_text();
        if s.is_some() {
            self.delete_selection();
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_text() {
        let mut t = TextEdit::new();
        t.insert_str("abc");
        assert_eq!(t.text(), "abc");
        assert_eq!(t.caret(), 3);
    }

    #[test]
    fn from_str_puts_caret_at_end() {
        let t = TextEdit::from_str("hello");
        assert_eq!(t.caret(), 5);
        assert_eq!(t.len(), 5);
    }

    #[test]
    fn unicode_is_char_indexed() {
        let mut t = TextEdit::from_str("щд"); // 2 символа, 4 байта
        assert_eq!(t.len(), 2);
        t.backspace();
        assert_eq!(t.text(), "щ");
        assert_eq!(t.caret(), 1);
    }

    #[test]
    fn move_left_right_home_end() {
        let mut t = TextEdit::from_str("abc");
        t.move_home(false);
        assert_eq!(t.caret(), 0);
        t.move_right(false);
        assert_eq!(t.caret(), 1);
        t.move_end(false);
        assert_eq!(t.caret(), 3);
        t.move_left(false);
        assert_eq!(t.caret(), 2);
    }

    #[test]
    fn insert_at_caret_middle() {
        let mut t = TextEdit::from_str("ac");
        t.move_home(false);
        t.move_right(false); // между a и c
        t.insert_str("b");
        assert_eq!(t.text(), "abc");
    }

    #[test]
    fn delete_at_caret() {
        let mut t = TextEdit::from_str("abc");
        t.move_home(false);
        t.delete();
        assert_eq!(t.text(), "bc");
    }

    #[test]
    fn shift_arrow_selects_and_insert_replaces() {
        let mut t = TextEdit::from_str("abcd");
        t.move_home(false);
        t.move_right(true); // выделено "a"
        t.move_right(true); // выделено "ab"
        assert_eq!(t.selected_text().as_deref(), Some("ab"));
        t.insert_str("X"); // замена выделения
        assert_eq!(t.text(), "Xcd");
    }

    #[test]
    fn select_all_and_cut() {
        let mut t = TextEdit::from_str("abc");
        t.select_all();
        assert_eq!(t.cut().as_deref(), Some("abc"));
        assert_eq!(t.text(), "");
        assert!(t.cut().is_none());
    }

    #[test]
    fn backspace_deletes_selection_not_char() {
        let mut t = TextEdit::from_str("abc");
        t.select_all();
        t.backspace();
        assert_eq!(t.text(), "");
    }
}
