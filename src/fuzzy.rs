//! Fuzzy matching for the finder: the query's chars in order, scored so matches in the file
//! name, at word starts and in runs rank first.

/// A path that matches, its score and the char indices of the matched chars.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub index: usize,
    pub score: i64,
    pub hits: Vec<usize>,
}

/// Scores `text` against `query` (lowercase compare unless the query has an uppercase char).
/// `None` when the query's chars don't all appear in order.
pub fn score(query: &str, text: &str) -> Option<(i64, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let smart = query.chars().any(|c| c.is_uppercase());
    let norm = |c: char| if smart { c } else { c.to_ascii_lowercase() };
    let t: Vec<char> = text.chars().collect();
    let q: Vec<char> = query.chars().map(norm).filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let name_start = t.iter().rposition(|c| *c == '/').map_or(0, |i| i + 1);
    // Greedy from the right inside the file name first, then anywhere: cheap and good enough.
    let hits = find(&t, &q, name_start, norm).or_else(|| find(&t, &q, 0, norm))?;
    let mut s: i64 = 0;
    let mut prev: Option<usize> = None;
    for &i in &hits {
        s += 10;
        if i >= name_start {
            s += 12;
        }
        let at_word = i == 0 || matches!(t[i - 1], '/' | '_' | '-' | '.' | ' ') || (t[i].is_uppercase() && t[i - 1].is_lowercase());
        if at_word {
            s += 18;
        }
        if let Some(p) = prev {
            if i == p + 1 {
                s += 16;
            } else {
                s -= (i - p) as i64 / 2;
            }
        }
        prev = Some(i);
    }
    if hits.first() == Some(&name_start) {
        s += 20;
    }
    // Shorter names first, then shorter paths.
    s -= (t.len() - name_start) as i64 / 2 + t.len() as i64 / 16;
    Some((s, hits))
}

fn find(t: &[char], q: &[char], from: usize, norm: impl Fn(char) -> char) -> Option<Vec<usize>> {
    let mut hits = Vec::with_capacity(q.len());
    let mut j = 0;
    for (i, &c) in t.iter().enumerate().skip(from) {
        if j < q.len() && norm(c) == q[j] {
            hits.push(i);
            j += 1;
        }
    }
    (j == q.len()).then_some(hits)
}

/// The best `limit` matches of `query` among `items`, best first. Ties keep the shorter path.
pub fn rank<S: AsRef<str>>(query: &str, items: &[S], limit: usize) -> (Vec<Match>, usize) {
    let mut out: Vec<Match> = items
        .iter()
        .enumerate()
        .filter_map(|(index, it)| score(query, it.as_ref()).map(|(score, hits)| Match { index, score, hits }))
        .collect();
    let total = out.len();
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| items[a.index].as_ref().len().cmp(&items[b.index].as_ref().len())));
    out.truncate(limit);
    (out, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_matches_beat_folder_matches() {
        let items = ["src/editor/motion.rs", "src/mode/other.rs", "tests/motions.rs"];
        let (m, total) = rank("mot", &items, 10);
        assert_eq!(total, 3);
        assert_eq!(items[m[0].index], "src/editor/motion.rs");
    }

    #[test]
    fn chars_must_appear_in_order() {
        assert!(score("tom", "motion.rs").is_none());
        assert_eq!(score("mrs", "m.rs").unwrap().1, vec![0, 2, 3]);
    }
}
