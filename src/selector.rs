// Re-export the Selector type from mdstore.
pub use mdstore::selector::Selector;

use crate::note::Note;

/// Return true if the note matches the selector.
pub fn matches_note(selector: &Selector, note: &Note) -> bool {
    match selector.namespace.as_str() {
        "tag" => note
            .frontmatter
            .tags
            .iter()
            .any(|t| t.as_str() == selector.value),
        "status" => note.frontmatter.status.to_string() == selector.value,
        "link" => note
            .frontmatter
            .links
            .iter()
            .any(|l| l.as_str() == selector.value),
        _ => false,
    }
}

/// Return true if the note matches every selector.
pub fn matches_all(selectors: &[Selector], note: &Note) -> bool {
    mdstore::selector::matches_all(selectors, note, matches_note)
}
