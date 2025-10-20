use std::collections::BTreeSet;

/// 管理書籤行號集合。 / Tracks bookmarked line numbers.
#[derive(Debug, Default, Clone)]
pub struct BookmarkManager {
    bookmarks: BTreeSet<usize>,
}

impl BookmarkManager {
    /// 加入書籤；若原先已存在則回傳 `false`。 / Inserts a bookmark, returning false if it already existed.
    pub fn add(&mut self, line: usize) -> bool {
        self.bookmarks.insert(line)
    }

    /// 移除書籤。 / Removes a bookmark and returns whether it was present.
    pub fn remove(&mut self, line: usize) -> bool {
        self.bookmarks.remove(&line)
    }

    /// 切換書籤狀態並回傳新的狀態。 / Toggles a bookmark and returns the new state.
    pub fn toggle(&mut self, line: usize) -> bool {
        if self.bookmarks.remove(&line) {
            false
        } else {
            self.bookmarks.insert(line);
            true
        }
    }

    /// 檢查是否存在。 / Checks whether a bookmark exists on the given line.
    pub fn is_bookmarked(&self, line: usize) -> bool {
        self.bookmarks.contains(&line)
    }

    /// 取得下一個書籤行。 / Finds the next bookmark after the provided line.
    pub fn next_after(&self, line: usize) -> Option<usize> {
        self.bookmarks.range((line + 1)..).next().copied()
    }

    /// 取得前一個書籤行。 / Finds the previous bookmark before the provided line.
    pub fn previous_before(&self, line: usize) -> Option<usize> {
        self.bookmarks.range(..line).next_back().copied()
    }

    /// 以遞增順序列出所有書籤。 / Iterates bookmarks in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.bookmarks.iter().copied()
    }

    /// 清除所有書籤。 / Clears the tracked bookmarks.
    pub fn clear(&mut self) {
        self.bookmarks.clear();
    }

    /// 書籤數量。 / Returns number of bookmarks.
    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    /// 是否為空。 / Indicates whether no bookmarks exist.
    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_and_iterate() {
        let mut manager = BookmarkManager::default();
        assert!(manager.toggle(5));
        assert!(manager.toggle(10));
        // Removing an existing bookmark should return false.
        // 移除既有書籤時應回傳 false。
        assert!(!manager.toggle(5));
        assert_eq!(manager.len(), 1);
        assert!(manager.is_bookmarked(10));
        let collected: Vec<_> = manager.iter().collect();
        assert_eq!(collected, vec![10]);
    }

    #[test]
    fn navigation_uses_sorted_entries() {
        let mut manager = BookmarkManager::default();
        manager.add(2);
        manager.add(5);
        manager.add(8);
        assert_eq!(manager.next_after(2), Some(5));
        assert_eq!(manager.next_after(8), None);
        assert_eq!(manager.previous_before(5), Some(2));
        assert_eq!(manager.previous_before(1), None);
    }
}
