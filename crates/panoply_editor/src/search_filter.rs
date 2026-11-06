//! The editor's search filter, as faery's `createSearchFilter`: what a user types in a list's
//! filter box, matched against each item's name.
//!
//! Spaces AND terms and commas OR them: `ab cd` matches names matching both `ab` and `cd`, and
//! `ab, cd` names matching either. A term matches a name that contains it, ignoring case, or whose
//! words start with its letters: each letter either starts a word or continues the word the letter
//! before it matched, so `bb` matches "BasicBlock" and `stvi` "stick-village". A word starts at the
//! name's start, at a capital, and after `-`, `_` or a space. No character in a term is special.

/// A parsed filter: its groups are ORed, and the terms within a group ANDed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchFilter {
    groups: Vec<Vec<Vec<char>>>,
}

impl SearchFilter {
    /// Parses `text`. An empty filter, or one of spaces and commas only, matches every name.
    pub fn new(text: &str) -> Self {
        let groups = text
            .split(',')
            .map(|group| {
                group
                    .split_whitespace()
                    .map(|term| term.chars().flat_map(char::to_lowercase).collect())
                    .collect::<Vec<Vec<char>>>()
            })
            .filter(|terms| !terms.is_empty())
            .collect();
        Self { groups }
    }

    pub fn matches(&self, name: &str) -> bool {
        if self.groups.is_empty() {
            return true;
        }
        let name = SearchedName::new(name);
        self.groups
            .iter()
            .any(|terms| terms.iter().all(|term| name.matches(term)))
    }
}

/// A name, lowercased, with where its words start.
struct SearchedName {
    chars: Vec<char>,
    word_start: Vec<bool>,
}

impl SearchedName {
    fn new(name: &str) -> Self {
        let mut chars = Vec::new();
        let mut word_start = Vec::new();
        let mut after_separator = true;
        for c in name.chars() {
            let starts = after_separator || c.is_uppercase();
            // A char may lowercase to several; only the first starts the word.
            for (i, lower) in c.to_lowercase().enumerate() {
                chars.push(lower);
                word_start.push(starts && i == 0);
            }
            after_separator = matches!(c, '-' | '_' | ' ');
        }
        Self { chars, word_start }
    }

    fn matches(&self, term: &[char]) -> bool {
        self.contains(term) || self.matches_word_starts(term)
    }

    fn contains(&self, term: &[char]) -> bool {
        term.is_empty() || self.chars.windows(term.len()).any(|window| window == term)
    }

    /// Whether `term`'s letters match in order, each starting a word or continuing the word the
    /// letter before matched. Tried from the end: `reachable[p]` says whether the rest of the term
    /// matches with its first letter at `p`, so each choice of where a letter goes is tried once.
    fn matches_word_starts(&self, term: &[char]) -> bool {
        let n = self.chars.len();
        if term.is_empty() || term.len() > n {
            return term.is_empty();
        }
        // For the last letter: anywhere it equals the name's letter.
        let mut reachable: Vec<bool> = (0..n)
            .map(|p| self.chars[p] == term[term.len() - 1])
            .collect();
        for &letter in term[..term.len() - 1].iter().rev() {
            // `later_start[p]`: some word start after `p` is reachable for the next letter.
            let mut later_start = vec![false; n + 1];
            for p in (0..n).rev() {
                later_start[p] = later_start[p + 1] || (self.word_start[p] && reachable[p]);
            }
            reachable = (0..n)
                .map(|p| {
                    self.chars[p] == letter
                        && ((p + 1 < n && reachable[p + 1]) || later_start[p + 1])
                })
                .collect();
        }
        (0..n).any(|p| self.word_start[p] && reachable[p])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(filter: &str, name: &str) -> bool {
        SearchFilter::new(filter).matches(name)
    }

    /// faery's tests, `createSearchFilter.test.ts`.
    #[test]
    fn substrings() {
        assert!(matches("ab", "ab"));
        assert!(matches("ab", "abc"));
        assert!(matches("ab", "abc 123"));
        assert!(!matches("ac", "abc"));
        assert!(matches("Alp", "Alpha"));
    }

    #[test]
    fn capitals_start_words() {
        assert!(matches("ab", "AlphaBeta"));
        assert!(!matches("ac", "AlphaBeta"));
        assert!(matches("bb", "BasicBlock"));
    }

    #[test]
    fn spaces_and_terms() {
        assert!(matches("ab de", "ab de"));
        assert!(matches("ab de", "abx dex"));
        assert!(!matches("ab de", "ab cc"));
        assert!(!matches("ab de", "cc de"));
    }

    #[test]
    fn commas_or_terms() {
        assert!(matches("ab,de", "ab de"));
        assert!(matches("ab,de", "abx dex"));
        assert!(matches("ab,de", "abx"));
        assert!(matches("ab,de", "dex"));
        assert!(!matches("ab,de", "cc"));
        assert!(matches("ab cd, xy", "xylophone"));
        assert!(!matches("ab cd, xy", "abacus"));
    }

    #[test]
    fn letters_continue_the_word_matched() {
        assert!(matches("stvi", "stick-village"));
        assert!(matches("hcab", "heron-creek-armory-basement"));
        assert!(matches("dcin", "demo_cave_inside"));
        // `t` does not follow `s` in "stick", and starts no word.
        assert!(!matches("tv", "stick-village"));
        // A letter cannot skip ahead within a word.
        assert!(!matches("sk", "stick-village"));
    }

    /// A first choice that fails must not hide a later one that works: `de` could take "demo"'s
    /// `d` and `e`, or the `d` of "demo" and the `e` of "exit".
    #[test]
    fn every_placement_is_tried() {
        assert!(matches("dex", "demo-exit"));
        assert!(matches("rre", "ruins-ruins-exit"));
    }

    #[test]
    fn case_is_ignored() {
        assert!(matches("STVI", "stick-village"));
        assert!(matches("caeth", "town.Caeth"));
    }

    #[test]
    fn no_character_is_special() {
        assert!(matches("town.c", "town.caeth"));
        assert!(!matches("town.c", "townscaeth"));
        assert!(!matches("a+", "aaa"));
        assert!(matches("a+", "a+b"));
        assert!(!matches("(", "abc"));
    }

    #[test]
    fn empty_matches_everything() {
        assert!(matches("", "anything"));
        assert!(matches(" , ", "anything"));
        assert!(!matches("x", ""));
    }
}
