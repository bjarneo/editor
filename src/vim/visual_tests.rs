//! Visual mode tests.

use super::tests::{at, cur, ed, feed, text};
use crate::editor::{Action, Mode, VisualKind};

#[test]
fn visual_char_selects_both_ends() {
    let mut e = ed("hello world");
    feed(&mut e, "vee");
    assert_eq!(e.mode, Mode::Visual(VisualKind::Char));
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(0));
    feed(&mut e, "d");
    assert_eq!(text(&e), "");
    assert_eq!(e.mode, Mode::Normal);
    assert_eq!(e.doc().view.anchor, None);
}

#[test]
fn visual_line_delete_and_yank() {
    let mut e = ed("a\nb\nc\nd\n");
    feed(&mut e, "jVjd");
    assert_eq!(text(&e), "a\nd\n");
    assert_eq!(cur(&e), (1, 0));
    let acts = feed(&mut e, "Vy");
    assert!(acts.contains(&Action::Copy("d\n".into())));
    assert!(e.regs.unnamed.linewise);
}

#[test]
fn visual_o_swaps_ends_and_text_objects_extend() {
    let mut e = ed("one two three");
    at(&mut e, 0, 5);
    feed(&mut e, "viw");
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(4));
    assert_eq!(cur(&e), (0, 6));
    feed(&mut e, "o");
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "<esc>vi(");
    assert_eq!(e.mode, Mode::Visual(VisualKind::Char), "no brackets keeps the selection as it was");
}

#[test]
fn visual_case_shift_and_join() {
    let mut e = ed("abc def");
    feed(&mut e, "veU");
    assert_eq!(text(&e), "ABC def");
    let mut e = ed("a\nb\nc");
    feed(&mut e, "Vj>");
    assert_eq!(text(&e), "    a\n    b\nc");
    feed(&mut e, "gv<lt>");
    assert_eq!(text(&e), "a\nb\nc", "gv reselects the last selection");
    feed(&mut e, "VjjJ");
    assert_eq!(text(&e), "a b c");
}

#[test]
fn visual_change_and_replace() {
    let mut e = ed("foo bar baz");
    at(&mut e, 0, 4);
    feed(&mut e, "vecqux<esc>");
    assert_eq!(text(&e), "foo qux baz");
    let mut e = ed("abc");
    feed(&mut e, "vlrx");
    assert_eq!(text(&e), "xxc");
}

#[test]
fn block_insert_writes_every_line() {
    let mut e = ed("abc\ndef\nghi");
    feed(&mut e, "<c-v>jjI// <esc>");
    assert_eq!(text(&e), "// abc\n// def\n// ghi");
    feed(&mut e, "u");
    assert_eq!(text(&e), "abc\ndef\nghi", "a block insert undoes as one step");
    let mut e = ed("ab\ncd\nef");
    feed(&mut e, "<c-v>jjA!<esc>");
    assert_eq!(text(&e), "a!b\nc!d\ne!f");
    let mut e = ed("ab\nc\nef");
    feed(&mut e, "<c-v>jj$A;<esc>");
    assert_eq!(text(&e), "ab;\nc;\nef;");
}

#[test]
fn block_delete_and_change() {
    let mut e = ed("abcd\nefgh\nijkl");
    at(&mut e, 0, 1);
    feed(&mut e, "<c-v>jld");
    assert_eq!(text(&e), "ad\neh\nijkl");
    let mut e = ed("abcd\nefgh");
    at(&mut e, 0, 1);
    feed(&mut e, "<c-v>jlcX<esc>");
    assert_eq!(text(&e), "aXd\neXh");
}

#[test]
fn visual_put_replaces_the_selection() {
    let mut e = ed("one two");
    feed(&mut e, "yiwwviwp");
    assert_eq!(text(&e), "one one");
    let mut e = ed("a\nb\nc");
    feed(&mut e, "yyjVp");
    assert_eq!(text(&e), "a\na\nc");
    let mut e = ed("a\nb");
    feed(&mut e, "yyjVp");
    assert_eq!(text(&e), "a\na\n", "a put over the last line lands after the line above");
}

#[test]
fn colon_from_visual_prefills_the_range() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "Vj:");
    assert_eq!(e.mode, Mode::Command);
    assert_eq!(e.cmd.as_ref().map(|c| c.text.as_str()), Some("'<,'>"));
    feed(&mut e, "d<cr>");
    assert_eq!(text(&e), "c");
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn search_from_visual_extends_the_selection() {
    let mut e = ed("a b c d");
    feed(&mut e, "v/c<cr>");
    assert_eq!(e.mode, Mode::Visual(VisualKind::Char));
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "d");
    assert_eq!(text(&e), " d");
}

#[test]
fn visual_comment_lines() {
    let mut e = ed("x\ny");
    feed(&mut e, "Vjgc");
    assert_eq!(text(&e), "// x\n// y");
}
