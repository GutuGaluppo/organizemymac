//! Aggregated storage tree built during a scan. Only folders (and files above a threshold) become
//! nodes, so memory stays proportional to the number of folders, not files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::filesystem::scanner::{ScanVisitor, VisitEntry};

#[derive(Debug, Clone)]
struct Node {
    name: String,
    parent: Option<u32>,
    is_dir: bool,
    size: u64,
    allocated: u64,
    files: u64,
    children: Vec<u32>,
}

#[derive(Debug)]
pub struct StorageTree {
    root: PathBuf,
    nodes: Vec<Node>,
    dirs: HashMap<PathBuf, u32>,
    /// Files at least this big get their own node (shown in the Space Map); smaller ones only add
    /// to their folder's total.
    min_file_node: u64,
    finished: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageNode {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub allocated: u64,
    pub files: u64,
    pub is_dir: bool,
    /// Size of the small files directly inside this folder that have no node of their own.
    pub loose_files_size: u64,
    pub has_children: bool,
    pub children: Option<Vec<StorageNode>>,
}

impl StorageTree {
    pub fn new(root: &Path, min_file_node: u64) -> Self {
        StorageTree { root: root.to_path_buf(), nodes: Vec::new(), dirs: HashMap::new(), min_file_node, finished: false }
    }

    fn push(&mut self, name: String, parent: Option<u32>, is_dir: bool) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(Node { name, parent, is_dir, size: 0, allocated: 0, files: 0, children: Vec::new() });
        if let Some(p) = parent {
            self.nodes[p as usize].children.push(id);
        }
        id
    }

    /// Adds each node's total to its parent. Nodes are created parent-first, so one reverse pass
    /// is enough.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        for i in (0..self.nodes.len()).rev() {
            if let Some(p) = self.nodes[i].parent {
                let (size, allocated, files) = (self.nodes[i].size, self.nodes[i].allocated, self.nodes[i].files);
                let parent = &mut self.nodes[p as usize];
                parent.size += size;
                parent.allocated += allocated;
                parent.files += files;
            }
        }
        self.finished = true;
    }

    pub fn root_id(&self) -> Option<u32> {
        if self.nodes.is_empty() { None } else { Some(0) }
    }

    pub fn path_of(&self, id: u32) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        while let Some(i) = cur {
            let n = &self.nodes[i as usize];
            if n.parent.is_some() {
                parts.push(n.name.as_str());
            }
            cur = n.parent;
        }
        let mut path = self.root.clone();
        for p in parts.iter().rev() {
            path.push(p);
        }
        path
    }

    /// Finds the node for a path inside the tree.
    pub fn find(&self, path: &Path) -> Option<u32> {
        if let Some(id) = self.dirs.get(path) {
            return Some(*id);
        }
        let parent = self.dirs.get(path.parent()?)?;
        let name = path.file_name()?.to_string_lossy();
        self.nodes[*parent as usize].children.iter().copied().find(|c| self.nodes[*c as usize].name == name)
    }

    /// A node with `depth` levels of children, largest first. At most `limit` children per level.
    pub fn view(&self, id: u32, depth: u32, limit: usize) -> Option<StorageNode> {
        let n = self.nodes.get(id as usize)?;
        let mut children: Vec<u32> = n.children.clone();
        children.sort_by_key(|c| std::cmp::Reverse(self.nodes[*c as usize].size));
        let child_total: u64 = n.children.iter().map(|c| self.nodes[*c as usize].size).sum();
        Some(StorageNode {
            id,
            name: if n.parent.is_none() { self.root.display().to_string() } else { n.name.clone() },
            path: self.path_of(id).display().to_string(),
            size: n.size,
            allocated: n.allocated,
            files: n.files,
            is_dir: n.is_dir,
            loose_files_size: n.size.saturating_sub(child_total),
            has_children: !n.children.is_empty(),
            children: (depth > 0).then(|| {
                children.iter().take(limit).filter_map(|c| self.view(*c, depth - 1, limit)).collect()
            }),
        })
    }

    /// Removes a node after its item was moved to the Trash, updating the totals above it.
    pub fn remove(&mut self, path: &Path) {
        let Some(id) = self.find(path) else { return };
        let (size, allocated, files) = {
            let n = &self.nodes[id as usize];
            (n.size, n.allocated, n.files)
        };
        let mut cur = self.nodes[id as usize].parent;
        if let Some(p) = cur {
            self.nodes[p as usize].children.retain(|c| *c != id);
        }
        while let Some(i) = cur {
            let n = &mut self.nodes[i as usize];
            n.size = n.size.saturating_sub(size);
            n.allocated = n.allocated.saturating_sub(allocated);
            n.files = n.files.saturating_sub(files);
            cur = n.parent;
        }
        self.dirs.retain(|p, _| !p.starts_with(path));
    }
}

impl ScanVisitor for StorageTree {
    fn visit(&mut self, e: &VisitEntry) {
        if e.depth == 0 {
            let id = self.push(String::new(), None, true);
            self.dirs.insert(e.path.to_path_buf(), id);
            return;
        }
        let Some(parent) = e.path.parent().and_then(|p| self.dirs.get(p)).copied() else { return };
        let name = e.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if e.meta.is_dir() {
            let id = self.push(name, Some(parent), true);
            self.dirs.insert(e.path.to_path_buf(), id);
            return;
        }
        let (size, allocated) = if e.first_link { (e.meta.size_logical, e.meta.size_allocated) } else { (0, 0) };
        if e.meta.is_file() && size >= self.min_file_node {
            let id = self.push(name, Some(parent), false);
            let n = &mut self.nodes[id as usize];
            n.size = size;
            n.allocated = allocated;
            n.files = 1;
        } else {
            let n = &mut self.nodes[parent as usize];
            n.size += size;
            n.allocated += allocated;
            n.files += u64::from(e.meta.is_file());
        }
    }
}
