//! The project index: every file under the root that git doesn't ignore, for the finder and
//! the file tree. Walked again when a changed path shows a file came or went.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use super::{Event, Out};

/// The index stops here, so a home folder opened by mistake stays quick.
const MAX_FILES: usize = 200_000;

pub fn start(root: PathBuf, out: Out) -> Sender<Vec<PathBuf>> {
    let (tx, rx) = channel::<Vec<PathBuf>>();
    std::thread::Builder::new()
        .name("editor-index".into())
        .spawn(move || {
            let mut files = walk(&root);
            out.send(Event::Files(files.clone()));
            while let Ok(first) = rx.recv() {
                // Coalesce a burst (a git checkout touches many files).
                std::thread::sleep(Duration::from_millis(150));
                let mut paths = first;
                for more in rx.try_iter() {
                    paths.extend(more);
                }
                let stale = paths.iter().any(|p| {
                    let Ok(rel) = p.strip_prefix(&root) else { return false };
                    let rel = rel.to_string_lossy().replace('\\', "/");
                    files.binary_search(&rel).is_ok() != p.is_file()
                });
                if stale {
                    files = walk(&root);
                    out.send(Event::Files(files.clone()));
                }
            }
        })
        .expect("start the index worker");
    tx
}

pub fn walk(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    for entry in walker.flatten() {
        if entry.file_type().is_some_and(|t| t.is_file())
            && let Ok(rel) = entry.path().strip_prefix(root)
        {
            files.push(rel.to_string_lossy().replace('\\', "/"));
            if files.len() >= MAX_FILES {
                break;
            }
        }
    }
    files.sort();
    files
}
