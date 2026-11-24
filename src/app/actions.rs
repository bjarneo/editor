//! What the keymaps ask for outside the text: saving, closing, opening files and pickers,
//! the clipboard and `:set`.

use std::path::PathBuf;

use editor::editor::{Action, Keymap};
use rune::Tx;

use super::{App, Overlay, PickKind, Settings};
use crate::services::{AfterSave, WriteKind};

impl App {
    pub fn run_actions(&mut self, actions: Vec<Action>, tx: &mut Tx) {
        let mut close_after_save = false;
        for a in &actions {
            if matches!(a, Action::Quit { force: false }) && actions.iter().any(|b| matches!(b, Action::Save(_))) {
                close_after_save = true;
            }
        }
        for a in actions {
            match a {
                Action::Save(path) => self.save_active(path, if close_after_save { AfterSave::Close } else { AfterSave::Nothing }),
                Action::Quit { .. } if close_after_save => {}
                Action::SaveAll => {
                    self.save_all(false);
                }
                Action::Quit { force } => self.close_active(force, true, tx),
                Action::QuitAll { force } => self.quit_all(force, tx),
                Action::Open(path) => self.open_path(path, None),
                Action::OpenAt(path, pos) => self.open_path(path, Some(pos)),
                Action::Rename(name) => self.rename(name),
                Action::CodeActions => self.code_actions(),
                Action::Filter { first, last, cmd } => self.filter(first, last, cmd),
                Action::Shell(cmd) => {
                    self.ed.message(format!(":!{cmd}  running"));
                    if let Some(svc) = &self.svc {
                        svc.shell(cmd);
                    }
                }
                Action::Reload => self.reload_active(),
                Action::FindFile => self.open_picker(PickKind::Files),
                Action::GrepProject => self.open_picker(PickKind::Grep),
                Action::PickBuffer => self.open_picker(PickKind::Buffers),
                Action::Palette => self.open_picker(PickKind::Commands),
                Action::ToggleTree => {
                    self.tree.show = !self.tree.show;
                    self.tree.focus = false;
                }
                Action::FocusTree => {
                    self.tree.show = true;
                    self.tree.focus = true;
                }
                Action::CloseBuffer => self.close_active(false, false, tx),
                Action::NextBuffer => self.cycle_buffer(1),
                Action::PrevBuffer => self.cycle_buffer(-1),
                Action::Diagnostics => self.show_diagnostics(),
                Action::GotoDefinition => self.goto_definition(),
                Action::Hover => self.hover_here(),
                Action::Settings => self.open_settings(),
                Action::LeaderMenu => self.overlay = Overlay::Leader(None),
                Action::Copy(text) => tx.copy(text),
                Action::Paste { before, count } => self.paste(before, count, tx),
                Action::SetOption(k, v) => self.set_option(&k, &v),
                Action::Message(m) => self.ed.message(m),
                Action::Error(e) => self.ed.error(e),
            }
        }
    }

    /// Writes the active buffer, or config.toml when the Settings tab shows.
    pub fn save_active(&mut self, path: Option<PathBuf>, then: AfterSave) {
        if self.on_settings {
            self.write_config();
            return;
        }
        if !self.ed.has_doc() {
            self.ed.error("E32: No file name");
            return;
        }
        if let Some(p) = path {
            let p = if p.is_absolute() { p } else { self.ed.root.join(p) };
            self.ed.doc_mut().buf.path = Some(p.clone());
            self.meta.entry(p.clone()).or_default().lang = Some(crate::highlight::lang_for(Some(&p)));
        }
        self.save_doc(self.ed.active, then);
    }

    fn save_doc(&mut self, i: usize, then: AfterSave) -> bool {
        let opts = self.ed.opts.save;
        let crlf_new = self.cfg.format.line_endings == "crlf";
        let doc = &mut self.ed.docs[i];
        if doc.buf.disk_mtime.is_none() && crlf_new {
            doc.buf.line_ending = editor::buffer::LineEnding::CrLf;
        }
        if let Some(crlf) = doc.buf.path.as_ref().and_then(|p| self.meta.get(p)).and_then(|m| m.ec.crlf) {
            doc.buf.line_ending = if crlf { editor::buffer::LineEnding::CrLf } else { editor::buffer::LineEnding::Lf };
        }
        let at = doc.view.cursor;
        match doc.buf.prepare_save_at(opts, at) {
            Ok((path, bytes, version)) => {
                let cursor = doc.view.cursor;
                doc.view.cursor = doc.buf.clamp(cursor, false);
                if let Some(svc) = &self.svc {
                    svc.save(path, bytes, version, then);
                }
                true
            }
            Err(e) => {
                self.ed.error(e.to_string());
                false
            }
        }
    }

    /// Writes every modified buffer. With `quit`, quits once the last write lands.
    pub fn save_all(&mut self, quit: bool) -> usize {
        let dirty: Vec<usize> =
            (0..self.ed.docs.len()).filter(|&i| self.ed.docs[i].buf.is_dirty() && self.ed.docs[i].buf.path.is_some()).collect();
        let n = dirty.iter().filter(|&&i| self.save_doc(i, AfterSave::Nothing)).count();
        if quit && n > 0 {
            self.quit_after_saves = Some(n);
        }
        n
    }

    /// `:q` closes the buffer and quits after the last one (`quit_last`); `SPC q` only closes.
    fn close_active(&mut self, force: bool, quit_last: bool, tx: &mut Tx) {
        if self.on_settings {
            self.close_settings();
            return;
        }
        if !self.ed.has_doc() {
            if quit_last {
                self.quit(tx);
            }
            return;
        }
        if !force && self.ed.buf().is_dirty() {
            self.ed.error("E37: No write since last change (add ! to override)");
            return;
        }
        let i = self.ed.active;
        if let (Some(path), Some(svc)) = (self.ed.docs[i].buf.path.clone(), &self.svc) {
            svc.lsp(crate::services::lsp::Job::Close { path: path.clone() });
            svc.unswap(path);
        }
        self.ed.close(i);
        self.ed.mode = self.ed.rest_mode();
        if self.ed.docs.is_empty() && quit_last && self.settings.is_none() {
            self.quit(tx);
            return;
        }
        if self.ed.docs.is_empty() && self.settings.is_some() {
            self.on_settings = true;
        }
        self.apply_doc_opts();
    }

    fn quit_all(&mut self, force: bool, tx: &mut Tx) {
        if !force {
            let dirty: Vec<String> = self.ed.docs.iter().filter(|d| d.buf.is_dirty()).map(|d| d.buf.name()).collect();
            if let Some(first) = dirty.first() {
                let more = if dirty.len() > 1 { format!(" and {} more", dirty.len() - 1) } else { String::new() };
                self.ed.error(format!("E37: No write since last change for \"{first}\"{more} (add ! to override)"));
                return;
            }
            if self.settings.as_ref().is_some_and(|_| self.cfg != self.disk_cfg) {
                self.ed.error("E37: Settings have changes. :w in the settings tab writes them, :qa! drops them");
                return;
            }
        }
        // The process ends right after this, before a worker could run: remove the swap files
        // of the edits `:qa!` drops here.
        for d in self.ed.docs.iter().filter(|d| d.buf.is_dirty()) {
            if let Some(p) = &d.buf.path {
                let _ = std::fs::remove_file(crate::services::io::swap_file(p));
            }
        }
        self.quit(tx);
    }

    fn reload_active(&mut self) {
        if !self.ed.has_doc() {
            return;
        }
        if let (Some(path), Some(svc)) = (self.ed.buf().path.clone(), &self.svc) {
            // `:e!` drops local edits: mark the buffer clean so the reload applies.
            let v = self.ed.buf().version();
            self.ed.doc_mut().buf.mark_saved(v, None);
            svc.unswap(path.clone());
            svc.read_disk(path);
        }
    }

    fn cycle_buffer(&mut self, by: isize) {
        let n = self.ed.docs.len() + usize::from(self.settings.is_some());
        if n == 0 {
            return;
        }
        // The Settings tab sits after the files.
        let cur = if self.on_settings { self.ed.docs.len() } else { self.ed.active };
        let next = (cur as isize + by).rem_euclid(n as isize) as usize;
        if next == self.ed.docs.len() {
            self.on_settings = true;
        } else {
            self.on_settings = false;
            self.ed.active = next;
            self.apply_doc_opts();
        }
        self.ed.mode = self.ed.rest_mode();
    }

    fn paste(&mut self, before: bool, count: usize, tx: &mut Tx) {
        tx.read_clipboard(move |s: &mut App, content, _tx| {
            let text = content.text().replace("\r\n", "\n");
            let (text, linewise) = if text.is_empty() || text == s.ed.regs.unnamed.text {
                (s.ed.regs.unnamed.text.clone(), s.ed.regs.unnamed.linewise)
            } else {
                let lw = text.ends_with('\n') && s.ed.keymap == Keymap::Vim;
                (text, lw)
            };
            if !text.is_empty() && s.ed.has_doc() {
                editor::vim::paste_text(&mut s.ed, &text, linewise, before, count);
                s.edited();
            }
        });
    }

    /// `:set name=value`, the palette's toggles and Settings' keymap switch.
    pub fn set_option(&mut self, k: &str, v: &str) {
        let on = !matches!(v, "false" | "0" | "off" | "no");
        match k {
            "keymap" | "keys.map" => self.cfg.keys.map = if v == "classic" { "classic".into() } else { "neovim".into() },
            "theme" => self.cfg.theme.mode = crate::theme::Source::parse(v).id().into(),
            "wrap" => self.cfg.editor.wrap = on,
            "number" | "nu" | "numbers" => {
                self.cfg.editor.numbers = if !on {
                    "off".into()
                } else if v == "true" {
                    "absolute".into()
                } else {
                    v.into()
                }
            }
            "relativenumber" | "rnu" => self.cfg.editor.numbers = if on { "hybrid".into() } else { "absolute".into() },
            "cursorline" | "cul" => self.cfg.editor.cursorline = on,
            "list" => self.cfg.editor.whitespace = on,
            "colorcolumn" | "cc" | "ruler" => self.cfg.editor.ruler = v.parse().unwrap_or(0),
            "mouse" => self.cfg.mouse.enabled = on && !v.is_empty(),
            "tabstop" | "ts" | "shiftwidth" | "sw" => self.cfg.indent.width = v.parse().unwrap_or(self.cfg.indent.width),
            "expandtab" | "et" => self.cfg.indent.spaces = on,
            "scrolloff" | "so" => self.cfg.editor.scrolloff = v.parse().unwrap_or(self.cfg.editor.scrolloff),
            _ => {
                self.ed.error(format!("E518: Unknown option: {k}"));
                return;
            }
        }
        self.config_changed();
    }

    /// Sends lines `first..=last` through `cmd`; the output replaces them when it arrives.
    fn filter(&mut self, first: usize, last: usize, cmd: String) {
        if !self.ed.has_doc() {
            return;
        }
        let buf = self.ed.buf();
        let Some(path) = buf.path.clone() else {
            self.ed.error("E32: No file name");
            return;
        };
        let last = last.min(buf.last_line());
        let a = buf.idx(editor::buffer::Pos::new(first, 0));
        let b = if last < buf.last_line() { buf.idx(editor::buffer::Pos::new(last + 1, 0)) } else { buf.len_chars() };
        let mut input = buf.slice(a, b);
        if !input.ends_with('\n') {
            input.push('\n');
        }
        let version = buf.version();
        self.ed.message(format!(":{},{}!{cmd}  running", first + 1, last + 1));
        if let Some(svc) = &self.svc {
            svc.filter(path, version, first, last, cmd, input);
        }
    }

    /// Closes tab `i` from the mouse: the close button or a middle-click. A tab with unsaved
    /// changes stays open and shows why.
    pub fn close_tab(&mut self, i: usize) {
        if self.ed.docs.get(i).is_some_and(|d| d.buf.is_dirty()) {
            self.ed.active = i;
            self.on_settings = false;
            self.ed.error("E37: No write since last change (save first, or :q! to drop it)");
            return;
        }
        if let (Some(path), Some(svc)) = (self.ed.docs.get(i).and_then(|d| d.buf.path.clone()), &self.svc) {
            svc.lsp(crate::services::lsp::Job::Close { path });
        }
        self.ed.close(i);
        self.apply_doc_opts();
    }

    pub fn open_settings(&mut self) {
        if self.settings.is_none() {
            self.settings = Some(Settings::default());
        }
        self.on_settings = true;
        self.overlay = Overlay::None;
        self.ed.mode = self.ed.rest_mode();
        self.ed.message(":Settings");
    }

    pub fn close_settings(&mut self) {
        self.settings = None;
        self.on_settings = false;
    }

    pub fn write_config(&mut self) {
        let text = self.cfg.to_toml();
        self.disk_cfg = self.cfg.clone();
        if let Some(svc) = &self.svc {
            svc.write(WriteKind::Config, crate::config::config_path(), text);
        }
    }
}
