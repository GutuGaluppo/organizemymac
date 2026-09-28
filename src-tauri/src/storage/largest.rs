use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::filesystem::scanner::{ScanVisitor, VisitEntry};
use crate::types::FileEntry;

struct BySize(FileEntry);

impl PartialEq for BySize {
    fn eq(&self, other: &Self) -> bool {
        self.0.size_logical == other.0.size_logical
    }
}
impl Eq for BySize {}
impl PartialOrd for BySize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BySize {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.size_logical.cmp(&other.0.size_logical)
    }
}

/// Keeps the N largest files seen during a scan (min-heap, O(log N) per file, N entries in memory).
pub struct LargestFiles {
    limit: usize,
    heap: BinaryHeap<Reverse<BySize>>,
}

impl LargestFiles {
    pub fn new(limit: usize) -> Self {
        LargestFiles { limit, heap: BinaryHeap::with_capacity(limit + 1) }
    }

    pub fn into_sorted(self) -> Vec<FileEntry> {
        let mut out: Vec<FileEntry> = self.heap.into_iter().map(|Reverse(BySize(e))| e).collect();
        out.sort_by_key(|e| Reverse(e.size_logical));
        out
    }
}

impl ScanVisitor for LargestFiles {
    fn visit(&mut self, e: &VisitEntry) {
        if !e.meta.is_file() || !e.first_link || self.limit == 0 {
            return;
        }
        if self.heap.len() >= self.limit {
            match self.heap.peek() {
                Some(Reverse(min)) if e.meta.size_logical <= min.0.size_logical => return,
                _ => {
                    self.heap.pop();
                }
            }
        }
        self.heap.push(Reverse(BySize(FileEntry::new(e.path, e.meta))));
    }
}
