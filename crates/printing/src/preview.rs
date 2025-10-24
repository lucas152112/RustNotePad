use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::job::PrintJobId;

/// Cache key for preview bitmaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrintPreviewKey {
    pub job_id: PrintJobId,
    pub page: u32,
    pub zoom_percent: u32,
}

/// Stored preview entry (e.g. rasterized WebP payload).
#[derive(Debug, Clone)]
pub struct PreviewEntry {
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: u32,
    pub data: Vec<u8>,
}

/// In-memory LRU-ish cache for preview pages.
#[derive(Debug, Default)]
pub struct PreviewCache {
    entries: HashMap<PrintPreviewKey, PreviewEntry>,
    order: Vec<PrintPreviewKey>,
    capacity: usize,
}

impl PreviewCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            capacity,
        }
    }

    pub fn insert(&mut self, key: PrintPreviewKey, entry: PreviewEntry) {
        if self.capacity == 0 {
            return;
        }
        let exists = self.entries.contains_key(&key);
        if !exists && self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.first().copied() {
                self.entries.remove(&oldest);
                self.order.remove(0);
            }
        }

        match self.entries.entry(key) {
            Entry::Occupied(mut occ) => {
                occ.insert(entry);
                self.touch(key);
            }
            Entry::Vacant(vac) => {
                vac.insert(entry);
                self.order.push(key);
            }
        }
    }

    pub fn get(&mut self, key: &PrintPreviewKey) -> Option<&PreviewEntry> {
        if self.entries.contains_key(key) {
            self.touch(*key);
            self.entries.get(key)
        } else {
            None
        }
    }

    pub fn remove_job(&mut self, job_id: PrintJobId) {
        self.entries.retain(|key, _| key.job_id != job_id);
        self.order.retain(|key| key.job_id != job_id);
    }

    pub fn invalidate_page_range(&mut self, job_id: PrintJobId, start: u32, end: u32) {
        self.entries
            .retain(|key, _| !(key.job_id == job_id && key.page >= start && key.page <= end));
        self.order
            .retain(|key| !(key.job_id == job_id && key.page >= start && key.page <= end));
    }

    fn touch(&mut self, key: PrintPreviewKey) {
        if let Some(idx) = self.order.iter().position(|k| *k == key) {
            let key = self.order.remove(idx);
            self.order.push(key);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
