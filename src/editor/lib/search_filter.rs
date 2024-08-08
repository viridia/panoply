/// Given a search input string, create a filter function that can be used to
/// select matching items. Understands word prefixes:
///
/// * "ab ac" - return items that have *both* a word starting with "ab" and "ac"
/// * "ab,ac" - return items that have *either* a word starting with "ab" or a word starting
///    with "ac".
///
/// Also understands camelCase, so 'bb' matches 'BrickButton'.
pub fn create_search_filter(search_text: &str) -> Box<dyn Fn(&str) -> bool> {
    if search_text.is_empty() {
        return Box::new(|_| true);
    }

    // Comma-separated list to search for multiple items.
    let comma_sep = search_text
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if comma_sep.len() > 1 {
        let filters = comma_sep
            .into_iter()
            .map(|term| create_search_filter(term))
            .collect::<Vec<_>>();
        return Box::new(move |name| filters.iter().any(|filter| filter(name)));
    }

    let search_text = comma_sep[0];
    let space_sep = search_text
        .split_whitespace()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if space_sep.len() > 1 {
        let filters = space_sep
            .into_iter()
            .map(|term| create_search_filter(term))
            .collect::<Vec<_>>();
        return Box::new(move |name| filters.iter().all(|filter| filter(name)));
    }

    let search_text = space_sep[0];
    let search_text = search_text
        .trim()
        .replace(&['[', ']', '(', ')', '\\'][..], "");
    let search_lower = search_text.to_lowercase();
    if search_lower == search_text {
        let search_camel = search_text
            .to_uppercase()
            .chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    format!(".*{}", c)
                } else {
                    c.to_string()
                }
            })
            .collect::<String>();
        let re = regex::Regex::new(&search_camel).unwrap();
        Box::new(move |name| name.to_lowercase().contains(&search_lower) || re.is_match(name))
    } else {
        let search_camel = search_text
            .chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    format!(".*{}", c)
                } else {
                    c.to_string()
                }
            })
            .collect::<String>();
        let re = regex::Regex::new(&search_camel).unwrap();
        Box::new(move |name| re.is_match(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_match() {
        let filter = create_search_filter("TestPattern");

        assert!(filter("TestPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_camel() {
        let filter = create_search_filter("tp");

        assert!(filter("TestPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_camel_upper() {
        let filter = create_search_filter("TP");

        assert!(filter("TestPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_camel_prefix() {
        let filter = create_search_filter("TePa");

        assert!(filter("TestPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_comma_sep() {
        let filter = create_search_filter("tp, sp");

        assert!(filter("TestPattern"));
        assert!(filter("StringPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_space_sep() {
        let filter = create_search_filter("tes pa");

        assert!(filter("TestPattern"));
        assert!(!filter("StringPattern"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_regex_match_with_lower_case() {
        let filter = create_search_filter("testpattern");

        assert!(filter("testpattern"));
        assert!(filter("TESTPATTERN"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_regex_match_with_mixed_case() {
        let filter = create_search_filter("TestPattern");

        assert!(filter("TestPattern"));
        assert!(!filter("testpattern"));
        assert!(!filter("TESTPATTERN"));
        assert!(!filter("AnotherPattern"));
    }

    #[test]
    fn test_regex_match_with_partial_match() {
        let filter = create_search_filter("test");

        assert!(filter("TestPattern"));
        assert!(filter("AnotherTestPattern"));
        assert!(!filter("AnotherPattern"));
    }
}
