//! The file tree: folders first, git status on each row, keys when it has focus.

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
    pub fn build(files: &[String]) -> TreeIndex {
        let mut children: HashMap<String, Vec<(String, bool)>> = HashMap::new();
        let mut dirs: HashSet<String> = HashSet::new();
        for f in files {
            let mut parent = String::new();
            let parts: Vec<&str> = f.split('/').collect();
            for (i, part) in parts.iter().enumerate() {
                let path = if parent.is_empty() { part.to_string() } else { format!("{parent}/{part}") };
                let last = i + 1 == parts.len();
                if last {
                    children.entry(parent.clone()).or_default().push((part.to_string(), false));
                } else if dirs.insert(path.clone()) {
                    children.entry(parent.clone()).or_default().push((part.to_string(), true));
                }
                parent = path;
            }
        }
        for list in children.values_mut() {
            list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
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

impl App {
    /// The git letter for a tree row: a file's own, or `M` on a folder with changes inside.
    pub fn git_letter(&self, row: &TreeRow) -> Option<char> {
        if !row.dir {
            return self.git.status.get(&row.path).copied();
        }
        let prefix = format!("{}/", row.path);
        self.git.status.keys().any(|k| k.starts_with(&prefix)).then_some('•')
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
