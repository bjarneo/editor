//! The file tree: folders first, git status on each row, keys when it has focus.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use editor::editor::{KeyCode, KeyInput};
use rune::Tx;

use super::App;

/// Each folder's children (by path relative to the root, `""` for the root), folders first.
#[derive(Default)]
pub struct TreeIndex {
    pub children: HashMap<String, Vec<(String, bool)>>,
}

/// One visible row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub dir: bool,
    pub open: bool,
}

impl TreeIndex {
    /// The tree of `files`. A large project takes tens of milliseconds, so the index worker
    /// builds it and the UI thread only swaps it in.
    pub fn build(files: &[String]) -> TreeIndex {
        let mut children: HashMap<String, Vec<(String, bool)>> = HashMap::new();
        let mut dirs: HashSet<&str> = HashSet::new();
        for f in files {
            let mut parent = "";
            let mut start = 0;
            for (i, _) in f.match_indices('/') {
                let dir = &f[..i];
                if dirs.insert(dir) {
                    push_child(&mut children, parent, &f[start..i], true);
                }
                parent = dir;
                start = i + 1;
            }
            push_child(&mut children, parent, &f[start..], false);
        }
        for list in children.values_mut() {
            list.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| caseless(&a.0, &b.0)).then_with(|| a.0.cmp(&b.0)));
        }
        TreeIndex { children }
    }

    /// The rows on screen with `open` folders expanded.
    pub fn rows(&self, open: &HashSet<String>) -> Vec<TreeRow> {
        let mut out = Vec::new();
        self.walk("", 0, open, &mut out);
        out
    }

    fn walk(&self, dir: &str, depth: usize, open: &HashSet<String>, out: &mut Vec<TreeRow>) {
        let Some(list) = self.children.get(dir) else { return };
        for (name, is_dir) in list {
            let path = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            let is_open = *is_dir && open.contains(&path);
            out.push(TreeRow { path: path.clone(), name: name.clone(), depth, dir: *is_dir, open: is_open });
            if is_open {
                self.walk(&path, depth + 1, open, out);
            }
        }
    }
}

/// Adds `name` to the children of `parent`, with no new key when the folder has one.
fn push_child(children: &mut HashMap<String, Vec<(String, bool)>>, parent: &str, name: &str, dir: bool) {
    match children.get_mut(parent) {
        Some(list) => list.push((name.to_string(), dir)),
        None => {
            children.insert(parent.to_string(), vec![(name.to_string(), dir)]);
        }
    }
}

/// Compares names as lowercase, without making lowercase copies.
fn caseless(a: &str, b: &str) -> Ordering {
    if a.is_ascii() && b.is_ascii() {
        return a.bytes().map(|c| c.to_ascii_lowercase()).cmp(b.bytes().map(|c| c.to_ascii_lowercase()));
    }
    a.chars().flat_map(char::to_lowercase).cmp(b.chars().flat_map(char::to_lowercase))
}

impl App {
    /// The git letter for a tree row: a file's own, or `M` on a folder with changes inside.
    pub fn git_letter(&self, row: &TreeRow) -> Option<char> {
        if !row.dir {
            return self.git.status.get(&row.path).copied();
        }
        self.git.dirs.contains(&row.path).then_some('•')
    }

    /// Opens or closes a folder, or opens a file.
    pub fn tree_press(&mut self, path: &str, dir: bool) {
        self.tree.sel = Some(path.to_string());
        if dir {
            if !self.tree.open.remove(path) {
                self.tree.open.insert(path.to_string());
            }
        } else {
            self.open_path(self.ed.root.join(path), None);
        }
    }

    /// Keys while the tree has focus. False passes the key on.
    pub fn tree_key(&mut self, k: KeyInput, tx: &mut Tx) -> bool {
        let rows = self.tree.index.rows(&self.tree.open);
        if rows.is_empty() {
            self.tree.focus = false;
            return false;
        }
        let cur = self.tree.sel.as_ref().and_then(|s| rows.iter().position(|r| &r.path == s)).unwrap_or(0);
        let go = |i: usize, app: &mut App| app.tree.sel = Some(rows[i.min(rows.len() - 1)].path.clone());
        match k.code {
            KeyCode::Char('j') | KeyCode::Down => go(cur + 1, self),
            KeyCode::Char('k') | KeyCode::Up => go(cur.saturating_sub(1), self),
            KeyCode::Char('g') => go(0, self),
            KeyCode::Char('G') => go(rows.len() - 1, self),
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('o') | KeyCode::Right => {
                let r = rows[cur].clone();
                if r.dir {
                    if !r.open || k.code == KeyCode::Enter || k.code == KeyCode::Char('o') {
                        self.tree_press(&r.path, true);
                    }
                } else {
                    self.tree_press(&r.path, false);
                    self.tree.focus = false;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                let r = &rows[cur];
                if r.dir && r.open {
                    self.tree.open.remove(&r.path);
                } else if let Some((parent, _)) = r.path.rsplit_once('/') {
                    self.tree.sel = Some(parent.to_string());
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => self.tree.focus = false,
            KeyCode::Char(c) if c == self.ed.vim.leader && self.is_vim() => {
                self.overlay = super::Overlay::Leader(None);
            }
            KeyCode::Char(':') => return false,
            _ => {
                let _ = tx;
                return k.cmd || k.ctrl;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_come_first_and_only_open_ones_expand() {
        let files: Vec<String> = ["README.md", "src/main.rs", "src/ui/tree.rs", ".gitignore"].iter().map(|s| s.to_string()).collect();
        let idx = TreeIndex::build(&files);
        let mut open = HashSet::new();
        let names: Vec<String> = idx.rows(&open).into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["src", ".gitignore", "README.md"]);
        open.insert("src".to_string());
        let rows = idx.rows(&open);
        assert_eq!(rows[1].name, "ui");
        assert_eq!(rows[2].name, "main.rs");
        assert_eq!(rows[2].depth, 1);
    }
}
