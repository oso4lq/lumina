use crate::decoder::{ext_lower, Decoder, StandardDecoder};
use std::path::{Path, PathBuf};

pub struct FolderCatalog {
    files: Vec<PathBuf>,
    current: usize,
}

impl FolderCatalog {
    /// Открыть папку файла `opened` и спозиционироваться на нём.
    pub fn open(opened: &Path) -> std::io::Result<Self> {
        let dir = opened.parent().unwrap_or_else(|| Path::new("."));
        let files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && StandardDecoder::supports(&ext_lower(p)))
            .collect();
        Ok(Self::sort_and_locate(files, opened))
    }

    /// Тестовый конструктор: список путей без обращения к ФС.
    #[cfg(test)]
    pub fn from_files_for_test(files: Vec<PathBuf>, opened: &Path) -> Self {
        let files: Vec<PathBuf> = files
            .into_iter()
            .filter(|p| StandardDecoder::supports(&ext_lower(p)))
            .collect();
        Self::sort_and_locate(files, opened)
    }

    fn sort_and_locate(mut files: Vec<PathBuf>, opened: &Path) -> Self {
        files.sort_by(|a, b| {
            let an = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let bn = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
            natord::compare(an, bn)
        });
        let opened_name = opened.file_name();
        let current = files
            .iter()
            .position(|p| p.file_name() == opened_name)
            .unwrap_or(0);
        Self { files, current }
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    pub fn current_path(&self) -> &Path {
        &self.files[self.current]
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Сдвинуться вперёд; вернуть true, если позиция изменилась.
    pub fn next(&mut self) -> bool {
        if self.current + 1 < self.files.len() {
            self.current += 1;
            true
        } else {
            false
        }
    }

    pub fn prev(&mut self) -> bool {
        if self.current > 0 {
            self.current -= 1;
            true
        } else {
            false
        }
    }

    pub fn go_first(&mut self) {
        self.current = 0;
    }

    pub fn go_last(&mut self) {
        if !self.files.is_empty() {
            self.current = self.files.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(names: &[&str], current_name: &str) -> FolderCatalog {
        let files: Vec<PathBuf> = names.iter().map(PathBuf::from).collect();
        FolderCatalog::from_files_for_test(files, Path::new(current_name))
    }

    #[test]
    fn natural_sort_orders_numerically() {
        let c = cat(&["IMG_10.jpg", "IMG_2.jpg", "IMG_1.jpg"], "IMG_1.jpg");
        let order: Vec<_> = c.files().iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(order, vec!["IMG_1.jpg", "IMG_2.jpg", "IMG_10.jpg"]);
    }

    #[test]
    fn current_points_to_opened_file() {
        let c = cat(&["a.jpg", "b.jpg", "c.jpg"], "b.jpg");
        assert_eq!(c.current_path().file_name().unwrap(), "b.jpg");
    }

    #[test]
    fn next_prev_saturate_at_edges() {
        let mut c = cat(&["a.jpg", "b.jpg"], "a.jpg");
        assert!(!c.prev());                       // уже в начале — без движения
        assert_eq!(c.current_index(), 0);
        assert!(c.next());                         // двинулись
        assert_eq!(c.current_index(), 1);
        assert!(!c.next());                        // уже в конце
        assert_eq!(c.current_index(), 1);
    }

    #[test]
    fn home_end_jump_to_edges() {
        let mut c = cat(&["a.jpg", "b.jpg", "c.jpg"], "b.jpg");
        c.go_first();
        assert_eq!(c.current_index(), 0);
        c.go_last();
        assert_eq!(c.current_index(), 2);
    }

    #[test]
    fn filters_unsupported_extensions() {
        // файл .txt не должен попасть в каталог
        let files = vec![PathBuf::from("a.jpg"), PathBuf::from("notes.txt"), PathBuf::from("b.png")];
        let c = FolderCatalog::from_files_for_test(files, Path::new("a.jpg"));
        assert_eq!(c.files().len(), 2);
    }
}
