//! The pickers that share the finder overlay: files (`SPC f`, ⌘P), project search (`SPC /`,
//! ⇧⌘F), open buffers (`SPC b`) and the command palette (⇧⌘P).

use std::path::PathBuf;

use editor::buffer::Pos;
use editor::editor::{Action, KeyCode, KeyInput};
use rune::Tx;

use super::{App, Overlay};
use crate::fuzzy::{self, Match};

/// Rows the finder ranks and draws at most.
const LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Files,
    Grep,
    Buffers,
    Commands,
    /// References, definitions or diagnostics from the language server.
    Locations,
    /// The language server's code actions at the cursor.
    Actions,
}

impl PickKind {
    pub fn title(self) -> &'static str {
        match self {
            PickKind::Files => "Find Files",
            PickKind::Grep => "Find in Project",
            PickKind::Buffers => "Buffers",
            PickKind::Commands => "Commands",
            PickKind::Locations => "Locations",
            PickKind::Actions => "Code Actions",
        }
    }
}

pub struct Picker {
    pub kind: PickKind,
    pub query: String,
    pub sel: usize,
    pub matches: Vec<Match>,
    /// How many candidates match, before the limit.
    pub total: usize,
    /// The candidates for Buffers and Commands (Files ranks the project index).
    pub items: Vec<String>,
    /// What each Buffers item opens: a document index.
    pub targets: Vec<usize>,
    /// The file the preview shows and its text, once the file worker read it.
    pub preview: Option<(PathBuf, String)>,
}

/// The command palette: a label and what it does.
pub const COMMANDS: &[(&str, &str)] = &[
    ("Find file", "find"),
    ("Find in project", "grep"),
    ("Save", "save"),
    ("Save all", "save-all"),
    ("Toggle file tree", "tree"),
    ("Open settings", "settings"),
    ("Close tab", "close"),
    ("Next tab", "next"),
    ("Previous tab", "prev"),
    ("Keymap: Neovim", "keymap=neovim"),
    ("Keymap: classic", "keymap=classic"),
    ("Theme: dark", "theme=dark"),
    ("Theme: light", "theme=light"),
    ("Theme: follow the OS", "theme=system"),
    ("Theme: follow Omarchy", "theme=omarchy"),
    ("Toggle soft wrap", "wrap"),
    ("Agent: list outside changes", "outside"),
    ("Agent: clear change marks", "clear-outside"),
    ("Agent: show the next proposal", "proposal"),
    ("Quit", "quit"),
];

impl Picker {
    pub fn new(kind: PickKind) -> Picker {
        Picker { kind, query: String::new(), sel: 0, matches: Vec::new(), total: 0, items: Vec::new(), targets: Vec::new(), preview: None }
    }

    /// Ranks the candidates against the query again.
    pub fn refilter(&mut self, files: &[String]) {
        let (matches, total) = match self.kind {
            PickKind::Files => fuzzy::rank(&self.query, files, LIMIT),
            // With nothing typed, buffers, commands and code actions keep their own order.
            PickKind::Buffers | PickKind::Commands | PickKind::Actions if self.query.is_empty() => {
                let all: Vec<Match> = (0..self.items.len().min(LIMIT)).map(|index| Match { index, score: 0, hits: Vec::new() }).collect();
                let n = self.items.len();
                (all, n)
            }
            PickKind::Buffers | PickKind::Commands | PickKind::Actions => fuzzy::rank(&self.query, &self.items, LIMIT),
            PickKind::Grep | PickKind::Locations => (Vec::new(), 0),
        };
        self.matches = matches;
        self.total = total;
        self.sel = self.sel.min(self.matches.len().saturating_sub(1));
    }

    /// Rows shown now: grep hits or locations for those kinds, else the ranked matches.
    pub fn len(&self, list_len: usize) -> usize {
        match self.kind {
            PickKind::Grep | PickKind::Locations => list_len.min(LIMIT * 5),
            _ => self.matches.len(),
        }
    }
}

impl App {
    pub fn open_picker(&mut self, kind: PickKind) {
        let mut p = Picker::new(kind);
        match kind {
            PickKind::Buffers => {
                for (i, d) in self.ed.docs.iter().enumerate() {
                    let name = d.buf.path.as_deref().map(|p| self.rel(p)).unwrap_or_else(|| "[No Name]".into());
                    let mark = if d.buf.is_dirty() { " [+]" } else { "" };
                    p.items.push(format!("{name}{mark}"));
                    p.targets.push(i);
                }
                p.sel = self.ed.active;
            }
            PickKind::Commands => p.items = COMMANDS.iter().map(|(l, _)| (*l).to_string()).collect(),
            PickKind::Grep => {
                self.grep.hits.clear();
                self.grep.error = None;
            }
            PickKind::Files | PickKind::Locations | PickKind::Actions => {}
        }
        p.refilter(&self.files);
        self.overlay = Overlay::Pick(Box::new(p));
        self.ask_preview();
    }

    /// Keys while a picker is open. Always takes the key.
    pub fn picker_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let Overlay::Pick(p) = &mut self.overlay else { return };
        let rows = p.len(if p.kind == PickKind::Locations { self.locations.len() } else { self.grep.hits.len() });
        let mut requery = false;
        match k.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Enter => {
                self.pick_accept(tx);
                return;
            }
            KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k') if k.code == KeyCode::Up || k.ctrl => p.sel = p.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j') if k.code == KeyCode::Down || k.ctrl => {
                p.sel = (p.sel + 1).min(rows.saturating_sub(1))
            }
            KeyCode::Tab if k.shift => p.sel = p.sel.saturating_sub(1),
            KeyCode::Tab => p.sel = (p.sel + 1).min(rows.saturating_sub(1)),
            KeyCode::PageUp => p.sel = p.sel.saturating_sub(10),
            KeyCode::PageDown => p.sel = (p.sel + 10).min(rows.saturating_sub(1)),
            KeyCode::Backspace => {
                p.query.pop();
                requery = true;
            }
            KeyCode::Char('u') if k.ctrl => {
                p.query.clear();
                requery = true;
            }
            KeyCode::Char('w') if k.ctrl => {
                let t = p.query.trim_end().len();
                let cut = p.query[..t].rfind([' ', '/', '_', '-', '.']).map_or(0, |i| i + 1);
                p.query.truncate(cut);
                requery = true;
            }
            KeyCode::Char(c) if !k.ctrl && !k.cmd && p.kind != PickKind::Locations => {
                p.query.push(c);
                requery = true;
            }
            KeyCode::Char('j') if p.kind == PickKind::Locations => p.sel = (p.sel + 1).min(rows.saturating_sub(1)),
            KeyCode::Char('k') if p.kind == PickKind::Locations => p.sel = p.sel.saturating_sub(1),
            _ => {}
        }
        if requery {
            p.sel = 0;
            if p.kind == PickKind::Grep {
                self.grep.generation += 1;
                self.grep.running = !p.query.is_empty();
                if let Some(svc) = &self.svc {
                    svc.grep(p.query.clone(), self.grep.generation);
                }
            } else {
                p.refilter(&self.files);
            }
        }
        self.ask_preview();
    }

    /// Lists code actions to pick from.
    pub fn open_actions(&mut self, titles: Vec<String>) {
        let mut p = Picker::new(PickKind::Actions);
        p.items = titles;
        p.refilter(&self.files);
        self.overlay = Overlay::Pick(Box::new(p));
    }

    /// The selected row's file and line, if it names one.
    pub fn picked(&self) -> Option<(PathBuf, Option<Pos>)> {
        let Overlay::Pick(p) = &self.overlay else { return None };
        match p.kind {
            PickKind::Files => {
                let m = p.matches.get(p.sel)?;
                Some((self.ed.root.join(&self.files[m.index]), None))
            }
            PickKind::Grep => {
                let h = self.grep.hits.get(p.sel)?;
                Some((self.ed.root.join(&h.path), Some(Pos::new(h.line, h.col))))
            }
            PickKind::Buffers => {
                let m = p.matches.get(p.sel)?;
                let i = *p.targets.get(m.index)?;
                Some((self.ed.docs.get(i)?.buf.path.clone()?, None))
            }
            PickKind::Locations => {
                let l = self.locations.get(p.sel)?;
                Some((l.path.clone(), Some(l.pos)))
            }
            PickKind::Commands | PickKind::Actions => None,
        }
    }

    /// Asks the file worker for the selected file's start, unless it is open or shown already.
    pub fn ask_preview(&mut self) {
        let Some((path, _)) = self.picked() else { return };
        let Overlay::Pick(p) = &mut self.overlay else { return };
        if p.preview.as_ref().is_some_and(|(pp, _)| *pp == path) {
            return;
        }
        if let Some(i) = self.ed.docs.iter().position(|d| d.buf.path.as_deref() == Some(path.as_path())) {
            p.preview = Some((path, self.ed.docs[i].buf.slice(0, 64 * 1024)));
            return;
        }
        if let Some(svc) = &self.svc {
            svc.preview(path);
        }
    }

    pub fn pick_accept(&mut self, tx: &mut Tx) {
        let Overlay::Pick(p) = &self.overlay else { return };
        let kind = p.kind;
        if kind == PickKind::Actions {
            let index = p.matches.get(p.sel).map(|m| m.index);
            self.overlay = Overlay::None;
            if let (Some(index), Some(svc)) = (index, &self.svc) {
                svc.lsp(crate::services::lsp::Job::RunAction { index });
            }
            return;
        }
        if kind == PickKind::Commands {
            let id = p.matches.get(p.sel).map(|m| COMMANDS[m.index].1);
            self.overlay = Overlay::None;
            if let Some(id) = id {
                self.run_command(id, tx);
            }
            return;
        }
        if kind == PickKind::Buffers
            && let Some(i) = p.matches.get(p.sel).and_then(|m| p.targets.get(m.index)).copied()
        {
            self.overlay = Overlay::None;
            self.ed.active = i.min(self.ed.docs.len().saturating_sub(1));
            self.on_settings = false;
            self.apply_doc_opts();
            return;
        }
        let picked = self.picked();
        self.overlay = Overlay::None;
        if let Some((path, at)) = picked {
            self.open_path(path, at);
        }
    }

    /// Runs a palette command by id.
    pub fn run_command(&mut self, id: &str, tx: &mut Tx) {
        let action = match id {
            "find" => Action::FindFile,
            "grep" => Action::GrepProject,
            "save" => Action::Save(None),
            "save-all" => Action::SaveAll,
            "tree" => Action::ToggleTree,
            "settings" => Action::Settings,
            "close" => Action::CloseBuffer,
            "next" => Action::NextBuffer,
            "prev" => Action::PrevBuffer,
            "wrap" => Action::SetOption("wrap".into(), (!self.cfg.editor.wrap).to_string()),
            "outside" => {
                self.show_outside();
                return;
            }
            "clear-outside" => {
                self.clear_outside(None);
                return;
            }
            "proposal" => {
                if let Some(p) = self.proposals.first().map(|p| p.path.clone()) {
                    self.open_path(p, None);
                }
                return;
            }
            "quit" => Action::QuitAll { force: false },
            other => match other.split_once('=') {
                Some((k, v)) => Action::SetOption(k.into(), v.into()),
                None => return,
            },
        };
        self.run_actions(vec![action], tx);
    }
}
