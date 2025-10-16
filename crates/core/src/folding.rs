/// 描述可折疊區塊。 / Represents a foldable region of lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldRegion {
    pub id: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub is_collapsed: bool,
}

/// 管理折疊狀態。 / Maintains fold regions for a document.
#[derive(Debug, Default, Clone)]
pub struct FoldTree {
    regions: Vec<FoldRegion>,
    next_id: usize,
}

impl FoldTree {
    /// 建立新的折疊區段並回傳識別碼。 / Defines a region and returns its identifier.
    pub fn define_region(&mut self, start_line: usize, end_line: usize) -> Option<usize> {
        if start_line >= end_line {
            return None;
        }
        if self.conflicts(start_line, end_line) {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.regions.push(FoldRegion {
            id,
            start_line,
            end_line,
            is_collapsed: false,
        });
        self.regions
            .sort_by_key(|region| (region.start_line, region.end_line));
        Some(id)
    }

    /// 設定折疊狀態。 / Sets collapsed state for the given region id.
    pub fn set_collapsed(&mut self, id: usize, collapsed: bool) -> bool {
        if let Some(region) = self.regions.iter_mut().find(|region| region.id == id) {
            region.is_collapsed = collapsed;
            true
        } else {
            false
        }
    }

    /// 切換折疊狀態並回傳新狀態。 / Toggles collapsed state, returning the new flag.
    pub fn toggle(&mut self, id: usize) -> Option<bool> {
        let region = self.regions.iter_mut().find(|region| region.id == id)?;
        region.is_collapsed = !region.is_collapsed;
        Some(region.is_collapsed)
    }

    /// 檢查行是否可見。 / Checks whether the given line should be visible.
    pub fn is_line_visible(&self, line: usize) -> bool {
        for region in &self.regions {
            if !region.is_collapsed {
                continue;
            }
            if line == region.start_line {
                return true;
            }
            if line > region.start_line && line <= region.end_line {
                return false;
            }
        }
        true
    }

    /// 列出可見行。 / Enumerates visible lines up to the provided total.
    pub fn visible_lines(&self, total_lines: usize) -> Vec<usize> {
        (0..total_lines)
            .filter(|&line| self.is_line_visible(line))
            .collect()
    }

    /// 移除所有折疊定義。 / Clears all regions.
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// 列出所有折疊區段。 / Returns all regions.
    pub fn regions(&self) -> &[FoldRegion] {
        &self.regions
    }

    fn conflicts(&self, start: usize, end: usize) -> bool {
        self.regions.iter().any(|region| {
            let overlaps = start < region.end_line && end > region.start_line;
            let partial_overlap =
                overlaps && !(start >= region.start_line && end <= region.end_line);
            partial_overlap
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_and_toggle_regions() {
        let mut tree = FoldTree::default();
        let id = tree.define_region(2, 5).unwrap();
        assert!(tree.set_collapsed(id, true));
        assert!(!tree.is_line_visible(3));
        assert!(tree.is_line_visible(2));
        assert!(tree.is_line_visible(6));
        assert_eq!(
            tree.visible_lines(8),
            vec![0, 1, 2, 6, 7]
        );
    }

    #[test]
    fn reject_conflicting_regions() {
        let mut tree = FoldTree::default();
        assert!(tree.define_region(1, 4).is_some());
        assert!(tree.define_region(4, 6).is_some());
        // partial overlap without nesting should fail
        assert!(tree.define_region(2, 5).is_none());
    }
}
