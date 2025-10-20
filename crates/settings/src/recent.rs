use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// 管理最近開啟檔案的清單。 / Maintains a bounded list of recently opened files.
#[derive(Debug, Clone)]
pub struct RecentFiles {
    capacity: usize,
    entries: VecDeque<PathBuf>,
}

impl RecentFiles {
    /// 建立指定容量的清單。 / Creates a history list with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    /// 依序列化資料還原最近檔案清單。 / Reconstructs the list from persisted entries.
    pub fn with_entries(capacity: usize, entries: Vec<PathBuf>) -> Self {
        let capacity = capacity.max(1);
        let mut deque: VecDeque<PathBuf> = entries.into_iter().collect();
        while deque.len() > capacity {
            deque.pop_back();
        }
        Self {
            capacity,
            entries: deque,
        }
    }

    /// 取得最大容量。 / Returns the maximum number of tracked entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 調整容量並修剪超出的紀錄。 / Adjusts capacity and trims excess entries.
    pub fn set_capacity(&mut self, capacity: usize) {
        let new_cap = capacity.max(1);
        self.capacity = new_cap;
        while self.entries.len() > new_cap {
            self.entries.pop_back();
        }
    }

    /// 加入或提升某個檔案路徑至清單頂端。 / Inserts or promotes a path to the front of the list.
    pub fn add(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.entries.retain(|existing| existing != &path);
        self.entries.push_front(path);
        while self.entries.len() > self.capacity {
            self.entries.pop_back();
        }
    }

    /// 移除指定路徑；若存在則回傳 `true`。 / Removes the given path and returns `true` if it existed.
    pub fn remove(&mut self, path: &Path) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|existing| existing.as_path() != path);
        initial_len != self.entries.len()
    }

    /// 清空清單。 / Clears all tracked entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 以不可變迭代器取得清單。 / Returns an iterator over the tracked entries.
    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.entries.iter()
    }

    /// 確認是否為空。 / Checks whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 目前的紀錄數。 / Current number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn add_promotes_and_limits_capacity() {
        let mut recent = RecentFiles::new(3);
        recent.add("a.txt");
        recent.add("b.txt");
        recent.add("c.txt");
        // Re-adding an existing path should promote it to the front.
        // 重新加入既有路徑時，應提升至清單前端。
        recent.add("b.txt");

        let collected: Vec<_> = recent.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(collected, vec!["b.txt", "c.txt", "a.txt"]);

        // Adding beyond capacity should evict the oldest entry.
        // 超出容量時應移除最舊的紀錄。
        recent.add("d.txt");
        let collected: Vec<_> = recent.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(collected, vec!["d.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn set_capacity_trims_entries() {
        let mut recent = RecentFiles::new(5);
        for ch in ['a', 'b', 'c', 'd'] {
            recent.add(format!("{ch}.txt"));
        }
        recent.set_capacity(2);
        let collected: Vec<_> = recent.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(collected, vec!["d.txt", "c.txt"]);
    }

    #[test]
    fn remove_and_clear() {
        let mut recent = RecentFiles::new(3);
        recent.add("x");
        recent.add("y");
        assert!(recent.remove(Path::new("x")));
        assert!(!recent.remove(Path::new("missing")));
        assert_eq!(recent.len(), 1);
        recent.clear();
        assert!(recent.is_empty());
    }

    #[test]
    fn with_entries_restores_state() {
        let paths = vec!["a", "b", "c"].into_iter().map(PathBuf::from).collect();
        let recent = RecentFiles::with_entries(2, paths);
        let collected: Vec<_> = recent.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(collected, vec!["a", "b"]);
        assert_eq!(recent.capacity(), 2);
    }
}
