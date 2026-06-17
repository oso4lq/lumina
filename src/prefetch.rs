//! In-memory LRU декодированных кадров с байтовым бюджетом — для мгновенного
//! перелистывания (префетч ±2). Чистое ядро, без GPU/rayon.
use crate::decoder::DecodedImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct PrefetchCache {
    map: HashMap<PathBuf, Arc<DecodedImage>>,
    lru: Vec<PathBuf>, // фронт — давний, хвост — свежий
    budget: u64,
    bytes: u64,
}

impl PrefetchCache {
    pub fn new(budget: u64) -> Self {
        Self { map: HashMap::new(), lru: Vec::new(), budget, bytes: 0 }
    }

    fn img_bytes(img: &DecodedImage) -> u64 {
        img.rgba.len() as u64
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.map.contains_key(path)
    }

    /// Получить кадр и пометить как свежеиспользованный (touch).
    pub fn get(&mut self, path: &Path) -> Option<Arc<DecodedImage>> {
        let img = self.map.get(path)?.clone();
        self.touch(path);
        Some(img)
    }

    fn touch(&mut self, path: &Path) {
        if let Some(pos) = self.lru.iter().position(|p| p == path) {
            let p = self.lru.remove(pos);
            self.lru.push(p);
        }
    }

    /// Вставить кадр; вытеснить LRU сверх бюджета. Возвращает вытесненные пути.
    pub fn insert(&mut self, path: PathBuf, img: Arc<DecodedImage>) -> Vec<PathBuf> {
        if let Some(old) = self.map.remove(&path) {
            self.bytes = self.bytes.saturating_sub(Self::img_bytes(&old));
            if let Some(pos) = self.lru.iter().position(|p| *p == path) {
                self.lru.remove(pos);
            }
        }
        self.bytes += Self::img_bytes(&img);
        self.map.insert(path.clone(), img);
        self.lru.push(path);
        let mut evicted = Vec::new();
        while self.bytes > self.budget && self.lru.len() > 1 {
            let old = self.lru.remove(0);
            if let Some(img) = self.map.remove(&old) {
                self.bytes = self.bytes.saturating_sub(Self::img_bytes(&img));
            }
            evicted.push(old);
        }
        evicted
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.lru.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(bytes: usize) -> Arc<DecodedImage> {
        Arc::new(DecodedImage { rgba: vec![0u8; bytes], width: 1, height: (bytes / 4) as u32 })
    }

    #[test]
    fn insert_and_get() {
        let mut c = PrefetchCache::new(1000);
        c.insert(PathBuf::from("a"), img(100));
        assert!(c.contains(Path::new("a")));
        assert!(c.get(Path::new("a")).is_some());
        assert!(c.get(Path::new("b")).is_none());
    }

    #[test]
    fn evicts_lru_over_byte_budget() {
        let mut c = PrefetchCache::new(250);
        c.insert(PathBuf::from("a"), img(100));
        c.insert(PathBuf::from("b"), img(100));
        let evicted = c.insert(PathBuf::from("c"), img(100)); // 300 > 250 → вытеснить "a"
        assert_eq!(evicted, vec![PathBuf::from("a")]);
        assert!(!c.contains(Path::new("a")));
        assert!(c.contains(Path::new("b")));
        assert!(c.contains(Path::new("c")));
    }

    #[test]
    fn get_touches_lru_order() {
        let mut c = PrefetchCache::new(250);
        c.insert(PathBuf::from("a"), img(100));
        c.insert(PathBuf::from("b"), img(100));
        let _ = c.get(Path::new("a")); // "a" освежён → теперь "b" старейший
        let evicted = c.insert(PathBuf::from("c"), img(100));
        assert_eq!(evicted, vec![PathBuf::from("b")]);
        assert!(c.contains(Path::new("a")));
    }

    #[test]
    fn clear_empties() {
        let mut c = PrefetchCache::new(1000);
        c.insert(PathBuf::from("a"), img(100));
        c.clear();
        assert!(!c.contains(Path::new("a")));
    }
}
