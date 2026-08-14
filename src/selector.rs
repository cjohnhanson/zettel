// Re-export the Selector type from mdstore.
pub use mdstore::selector::Selector;

use crate::note::Note;

pub const NAMESPACES: [&str; 3] = ["tag", "provenance", "link"];

/// Parse and validate one selector. A colonless value or an unknown
/// namespace is an error, so a typo cannot silently disable the filter
/// and list everything.
pub fn parse_selector(s: &str) -> crate::Result<Selector> {
    let sel = Selector::parse(s).ok_or_else(|| crate::Error::InvalidSelector(s.into()))?;
    if !NAMESPACES.contains(&sel.namespace.as_str()) {
        return Err(crate::Error::InvalidSelector(s.into()));
    }
    Ok(sel)
}

/// Return true if the note matches the selector.
pub fn matches_note(selector: &Selector, note: &Note) -> bool {
    match selector.namespace.as_str() {
        "tag" => note
            .frontmatter
            .tags
            .iter()
            .any(|t| t.as_str() == selector.value),
        "provenance" => crate::provenance::note_matches_tokens(
            note.frontmatter.provenance.as_deref(),
            &note.body,
            &[selector.value.as_str()],
        ),
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
