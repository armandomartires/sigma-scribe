//! Small text helpers shared by the editor and the status bar.

/// Words as the status bar counts them: runs of non-whitespace.
#[must_use]
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Cleans a note name as the brief asks ("names are cleaned on rename"):
/// trims, collapses inner whitespace, drops path separators and control
/// characters, and falls back to `untitled`.
#[must_use]
pub fn clean_name(raw: &str) -> String {
    let filtered: String = raw
        .chars()
        .map(|c| if c == '/' || c == '\\' || c.is_control() { ' ' } else { c })
        .collect();
    let joined = filtered.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = joined.trim_start_matches('.').trim();
    if trimmed.is_empty() {
        "untitled".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("  one two\nthree\t four "), 4);
    }

    #[test]
    fn cleans_names() {
        assert_eq!(clean_name("  my   note "), "my note");
        assert_eq!(clean_name("../etc/passwd"), "etc passwd");
        assert_eq!(clean_name("..."), "untitled");
        assert_eq!(clean_name("a\u{0}b"), "a b");
    }
}
