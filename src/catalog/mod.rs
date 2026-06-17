use crate::decoder::{ext_lower, supported};
use std::path::{Path, PathBuf};

/// Индекс курсора в новом списке после пересбора: позиция файла с тем же именем,
/// что у прежнего текущего; если файл исчез — последний валидный индекс (или 0 для пустого).
pub fn relocate(files: &[PathBuf], prev_current: &Path) -> usize {
    if files.is_empty() {
        return 0;
    }
    let prev_name = prev_current.file_name();
    files
        .iter()
        .position(|p| p.file_name() == prev_name)
        .unwrap_or(files.len() - 1)
}

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
            .filter(|p| p.is_file() && supported(&ext_lower(p)))
            .collect();
        Ok(Self::sort_and_locate(files, opened))
    }

    /// Тестовый конструктор: список путей без обращения к ФС.
    #[cfg(test)]
    pub fn from_files_for_test(files: Vec<PathBuf>, opened: &Path) -> Self {
        let files: Vec<PathBuf> = files
            .into_iter()
            .filter(|p| supported(&ext_lower(p)))
            .collect();
        Self::sort_and_locate(files, opened)
    }

    /// Чистое ядро пересбора: отфильтрованный список → отсортированный + индекс курсора
    /// по прежнему пути. Без обращения к ФС (тестируемо).
    fn rebuild_core(mut files: Vec<PathBuf>, prev_current: &Path) -> (Vec<PathBuf>, usize) {
        files.sort_by(|a, b| {
            let an = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let bn = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
            natord::compare(an, bn)
        });
        let current = relocate(&files, prev_current);
        (files, current)
    }

    /// Пере-сканировать папку (повторное чтение ФС), сохранив текущий файл по пути.
    /// Возвращает true, если набор файлов или индекс изменились.
    pub fn refresh(&mut self) -> std::io::Result<bool> {
        let prev_current = self.files.get(self.current).cloned().unwrap_or_default();
        let dir = prev_current.parent()
            .or_else(|| self.files.first().and_then(|p| p.parent()))
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let scanned: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && supported(&ext_lower(p)))
            .collect();
        let (files, current) = Self::rebuild_core(scanned, &prev_current);
        let changed = files != self.files || current != self.current;
        self.files = files;
        self.current = current;
        Ok(changed)
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

    /// Перейти к индексу. true, если индекс валиден и отличается от текущего.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index < self.files.len() && index != self.current {
            self.current = index;
            true
        } else {
            false
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
    fn rebuild_sorts_and_relocates() {
        // моделируем результат повторного чтения папки: новый набор + прежний текущий путь
        let new_files: Vec<PathBuf> = ["IMG_1.jpg", "IMG_2.jpg", "IMG_10.jpg", "IMG_3.jpg"]
            .iter().map(PathBuf::from).collect();
        let (sorted, current) = FolderCatalog::rebuild_core(new_files, Path::new("IMG_2.jpg"));
        let names: Vec<&str> = sorted.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["IMG_1.jpg", "IMG_2.jpg", "IMG_3.jpg", "IMG_10.jpg"]); // натуральная сортировка
        assert_eq!(current, 1); // IMG_2.jpg
    }

    #[test]
    fn relocate_keeps_existing_file() {
        let files: Vec<PathBuf> = ["a.jpg", "b.jpg", "c.jpg"].iter().map(PathBuf::from).collect();
        // текущий был b.jpg → индекс 1
        assert_eq!(relocate(&files, Path::new("b.jpg")), 1);
    }

    #[test]
    fn relocate_missing_file_clamps_to_last() {
        let files: Vec<PathBuf> = ["a.jpg", "b.jpg"].iter().map(PathBuf::from).collect();
        // z.jpg отсутствует → последний валидный (len-1 = 1)
        assert_eq!(relocate(&files, Path::new("z.jpg")), 1);
    }

    #[test]
    fn relocate_empty_is_zero() {
        let files: Vec<PathBuf> = Vec::new();
        assert_eq!(relocate(&files, Path::new("a.jpg")), 0);
    }

    #[test]
    fn filters_unsupported_extensions() {
        // файл .txt не должен попасть в каталог
        let files = vec![PathBuf::from("a.jpg"), PathBuf::from("notes.txt"), PathBuf::from("b.png")];
        let c = FolderCatalog::from_files_for_test(files, Path::new("a.jpg"));
        assert_eq!(c.files().len(), 2);
    }
}
