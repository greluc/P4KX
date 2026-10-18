use std::ops::Range;

use crate::sort::compare_ignoring_case;

/// Identifies a folder of an [`crate::EntryIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FolderId(pub(crate) usize);

/// One folder of the path tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    name: String,
    path: String,
    children: Vec<FolderId>,
    entries: Range<u32>,
}

pub(crate) static NO_FOLDER: Folder = Folder {
    name: String::new(),
    path: String::new(),
    children: Vec::new(),
    entries: 0..0,
};

impl Folder {
    const fn new(name: String, path: String, first_entry: u32) -> Self {
        Self {
            name,
            path,
            children: Vec::new(),
            entries: first_entry..first_entry,
        }
    }

    /// Returns the folder name, empty for the root.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path of the folder inside the Archive, with backslash separators.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the subfolders, sorted by name ignoring case.
    #[must_use]
    pub fn children(&self) -> &[FolderId] {
        &self.children
    }

    /// Returns the number of Entries in this folder and all folders below it.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the ids of the Entries in this folder and below, a contiguous range in path order.
    pub(crate) fn entries(&self) -> Range<u32> {
        self.entries.clone()
    }
}

/// Builds the folders of `paths`, which are sorted byte by byte; the root comes first.
pub(crate) fn build<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<Folder> {
    let mut folders = vec![Folder::new(String::new(), String::new(), 0)];
    let mut open = vec![0];
    let mut next_id = 0;
    for path in paths {
        let parts = path
            .rsplit_once('\\')
            .map_or_else(Vec::new, |(dir, _)| dir.split('\\').collect());
        let kept = common_depth(&folders, &open, &parts);
        close(&mut folders, &mut open, kept + 1, next_id);
        for part in parts.iter().skip(kept) {
            add_child(&mut folders, &mut open, part, next_id);
        }
        next_id += 1;
    }
    close(&mut folders, &mut open, 0, next_id);
    sort_children(&mut folders);
    folders
}

fn common_depth(folders: &[Folder], open: &[usize], parts: &[&str]) -> usize {
    open.iter()
        .skip(1)
        .zip(parts)
        .take_while(|&(&folder, part)| folders.get(folder).is_some_and(|f| f.name == *part))
        .count()
}

fn close(folders: &mut [Folder], open: &mut Vec<usize>, keep: usize, end: u32) {
    while open.len() > keep {
        if let Some(folder) = open.pop().and_then(|folder| folders.get_mut(folder)) {
            folder.entries.end = end;
        }
    }
}

fn add_child(folders: &mut Vec<Folder>, open: &mut Vec<usize>, name: &str, first_entry: u32) {
    let id = folders.len();
    let Some(parent) = open.last().and_then(|&parent| folders.get_mut(parent)) else {
        return;
    };
    parent.children.push(FolderId(id));
    let path = if parent.path.is_empty() {
        name.to_owned()
    } else {
        format!("{}\\{name}", parent.path)
    };
    folders.push(Folder::new(name.to_owned(), path, first_entry));
    open.push(id);
}

fn sort_children(folders: &mut [Folder]) {
    let names: Vec<String> = folders.iter().map(|folder| folder.name.clone()).collect();
    let name = |id: &FolderId| names.get(id.0).map_or("", String::as_str);
    for folder in folders.iter_mut() {
        folder
            .children
            .sort_by(|a, b| compare_ignoring_case(name(a), name(b)));
    }
}
