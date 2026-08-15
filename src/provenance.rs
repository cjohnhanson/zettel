//! The zettel provenance vocabulary on top of mdstore's generic markers.
//!
//! mdstore parses the shape (`origin[:qualifier] [key=value ...]`); this
//! module decides what is valid:
//!
//! - `human[:name]` — the qualifier names the author.
//! - `agent[:kind]` — the kind is `summary`, `index`, or `inference`.
//! - `citation[:source]` — verbatim quoted material; the source is a note
//!   ID or a short key, with a `src=` attribute for URLs.
//! - No provenance means **unknown**. Unknown is never upgraded to human.
//!
//! The `reviewed=DATE` attribute marks human approval of an agent span.
//! Only `zettel note review` writes it.

use mdstore::provenance::{parse_spans, Marker};

use crate::error::{Error, Result};

pub const ORIGIN_HUMAN: &str = "human";
pub const ORIGIN_AGENT: &str = "agent";
pub const ORIGIN_CITATION: &str = "citation";
pub const AGENT_KINDS: [&str; 3] = ["summary", "index", "inference"];
pub const ATTR_REVIEWED: &str = "reviewed";
pub const ATTR_REVIEWER: &str = "reviewer";

/// Convert an mdstore error without doubling the "invalid provenance"
/// prefix its Display already carries.
pub fn from_mdstore(e: mdstore::Error) -> Error {
    match e {
        mdstore::Error::InvalidProvenance(msg) => Error::InvalidProvenance(msg),
        // A store-layer failure is not a provenance problem. Calling it
        // one sent a reader looking at the wrong thing.
        other => Error::Store(other.to_string()),
    }
}

/// Parse a spec string and validate it against the vocabulary.
pub fn parse_spec(spec: &str) -> Result<Marker> {
    let marker = Marker::parse(spec).map_err(from_mdstore)?;
    validate_marker(&marker)?;
    Ok(marker)
}

/// Parse a spec a person supplied, and refuse a review stamp in it.
///
/// A stamp records that a human read the text and approved it. Only
/// `note review --approve` writes one. Accepting one here would let a
/// caller declare its own content reviewed without anybody reviewing
/// it, which is the one claim the whole model rests on.
pub fn parse_authored_spec(spec: &str) -> Result<Marker> {
    let marker = parse_spec(spec)?;
    if marker.attr(ATTR_REVIEWED).is_some() || marker.attr(ATTR_REVIEWER).is_some() {
        return Err(Error::InvalidProvenance(
            "a review stamp comes from 'note review --approve', not from a written spec"
                .into(),
        ));
    }
    Ok(marker)
}

/// Validate a parsed marker against the vocabulary.
pub fn validate_marker(marker: &Marker) -> Result<()> {
    match marker.origin.as_str() {
        ORIGIN_HUMAN | ORIGIN_CITATION => Ok(()),
        ORIGIN_AGENT => match marker.qualifier.as_deref() {
            None => Ok(()),
            Some(kind) if AGENT_KINDS.contains(&kind) => Ok(()),
            Some(kind) => Err(Error::InvalidProvenance(format!(
                "'{kind}' is not an agent kind; use summary, index, or inference"
            ))),
        },
        origin => Err(Error::InvalidProvenance(format!(
            "'{origin}' is not an origin; use human, agent, or citation"
        ))),
    }
}

/// A body span with its effective provenance.
///
/// `marker: None` means unknown: the span is unmarked and the note has no
/// default provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSpan {
    /// The 1-based position among the raw body spans. `note review
    /// --approve` takes this number. Positions can have gaps: a blank
    /// separator line between markers is a raw span, but it carries no
    /// content, so it is not resolved.
    pub index: usize,
    pub marker: Option<Marker>,
    /// True when the provenance comes from the note's frontmatter default
    /// (or is unknown), not from an inline marker.
    pub from_default: bool,
    pub text: String,
}

/// A raw span with no content: an unmarked whitespace-only run between
/// markers. It exists for exact round-trips and means nothing else.
pub fn is_separator(marker: Option<&Marker>, text: &str) -> bool {
    marker.is_none() && text.trim().is_empty()
}

/// Split a body into spans and resolve each against the note default.
///
/// Errors on an invalid default spec or an invalid body marker.
pub fn resolve_spans(default_spec: Option<&str>, body: &str) -> Result<Vec<ResolvedSpan>> {
    let default = default_spec.map(parse_spec).transpose()?;
    let spans = parse_spans(body).map_err(from_mdstore)?;
    let mut resolved = Vec::new();
    for (i, span) in spans.into_iter().enumerate() {
        if is_separator(span.marker.as_ref(), &span.text) {
            continue;
        }
        match span.marker {
            Some(marker) => {
                validate_marker(&marker)?;
                resolved.push(ResolvedSpan {
                    index: i + 1,
                    marker: Some(marker),
                    from_default: false,
                    text: span.text,
                });
            }
            None => resolved.push(ResolvedSpan {
                index: i + 1,
                marker: default.clone(),
                from_default: true,
                text: span.text,
            }),
        }
    }
    Ok(resolved)
}

/// Like `resolve_spans`, but a note that does not parse degrades to one
/// unknown span instead of failing. Repo-wide commands use this so one
/// bad note cannot break them; `zettel check` reports the bad note.
pub fn resolve_spans_lenient(default_spec: Option<&str>, body: &str) -> Vec<ResolvedSpan> {
    resolve_spans(default_spec, body).unwrap_or_else(|_| {
        vec![ResolvedSpan {
            index: 1,
            marker: None,
            from_default: true,
            text: body.to_string(),
        }]
    })
}

/// Show a provenance for display: the spec string, or `unknown`.
pub fn display(marker: Option<&Marker>) -> String {
    match marker {
        Some(m) => m.to_string(),
        None => "unknown".to_string(),
    }
}

/// Validate one filter token: `unknown`, `reviewed`, or `origin[:qualifier]`.
pub fn validate_filter_token(token: &str) -> Result<()> {
    if token == "unknown" || token == "reviewed" {
        return Ok(());
    }
    let marker = Marker::parse(token).map_err(from_mdstore)?;
    if !marker.attrs.is_empty() {
        return Err(Error::InvalidProvenance(format!(
            "'{token}' is not a filter token; use unknown, reviewed, or origin[:qualifier]"
        )));
    }
    validate_marker(&marker)
}

/// Return true if the provenance matches the filter token.
///
/// - `unknown` matches a span with no provenance.
/// - `reviewed` matches a span with a `reviewed=` attribute.
/// - `origin` matches every span with that origin.
/// - `origin:qualifier` matches on both.
pub fn matches_token(marker: Option<&Marker>, token: &str) -> bool {
    match token {
        "unknown" => marker.is_none(),
        "reviewed" => marker.is_some_and(|m| m.attr(ATTR_REVIEWED).is_some()),
        _ => marker.is_some_and(|m| match token.split_once(':') {
            None => m.origin == token,
            Some((origin, qualifier)) => {
                m.origin == origin && m.qualifier.as_deref() == Some(qualifier)
            }
        }),
    }
}

/// Return true if the provenance matches any of the comma-separated tokens.
pub fn matches_any(marker: Option<&Marker>, tokens: &[&str]) -> bool {
    tokens.iter().any(|t| matches_token(marker, t))
}

/// Return true for an agent span with no `reviewed=` attribute.
pub fn is_unreviewed_agent(marker: Option<&Marker>) -> bool {
    marker.is_some_and(|m| m.origin == ORIGIN_AGENT && m.attr(ATTR_REVIEWED).is_none())
}

/// The sources cited by a note: the qualifier of every citation-origin
/// span, the default included. A source that resolves to a note ID joins
/// the link graph; any other source is an external key.
pub fn citation_refs(default_spec: Option<&str>, body: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for span in resolve_spans_lenient(default_spec, body) {
        if let Some(m) = &span.marker
            && m.origin == ORIGIN_CITATION
            && let Some(q) = &m.qualifier
            && !refs.contains(q)
        {
            refs.push(q.clone());
        }
    }
    refs
}

/// The effective provenance of a whole note against filter tokens: any
/// span matches, or, when the body has no spans (an empty note), the
/// note's default provenance matches.
pub fn note_matches_tokens(default_spec: Option<&str>, body: &str, tokens: &[&str]) -> bool {
    let spans = resolve_spans_lenient(default_spec, body);
    if spans.is_empty() {
        let default = default_spec.and_then(|s| parse_spec(s).ok());
        return matches_any(default.as_ref(), tokens);
    }
    spans.iter().any(|s| matches_any(s.marker.as_ref(), tokens))
}

/// Return true if the note holds any unreviewed agent content. An empty
/// note counts through its default provenance.
pub fn note_has_unreviewed(default_spec: Option<&str>, body: &str) -> bool {
    let spans = resolve_spans_lenient(default_spec, body);
    if spans.is_empty() {
        let default = default_spec.and_then(|s| parse_spec(s).ok());
        return is_unreviewed_agent(default.as_ref());
    }
    spans.iter().any(|s| is_unreviewed_agent(s.marker.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_specs_parse() {
        for spec in [
            "human",
            "human:cody",
            "agent",
            "agent:summary",
            "agent:index",
            "agent:inference",
            "citation",
            "citation:a3f2",
            "citation src=https://example.com",
            "agent:summary reviewed=2026-08-14",
        ] {
            assert!(parse_spec(spec).is_ok(), "{spec} must be valid");
        }
    }

    #[test]
    fn invalid_specs_error() {
        for spec in ["robot", "agent:guess", "human extra", ""] {
            assert!(parse_spec(spec).is_err(), "{spec} must be invalid");
        }
    }

    #[test]
    fn unmarked_text_takes_the_default() {
        let spans = resolve_spans(Some("agent:summary"), "plain text").unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].from_default);
        assert_eq!(display(spans[0].marker.as_ref()), "agent:summary");
    }

    #[test]
    fn no_default_means_unknown() {
        let spans = resolve_spans(None, "plain text").unwrap();
        assert_eq!(spans[0].marker, None);
        assert_eq!(display(spans[0].marker.as_ref()), "unknown");
    }

    #[test]
    fn marked_spans_keep_their_own_provenance() {
        let body = "summary text\n<!-- prov human:cody -->\nconfirmed\n<!-- /prov -->";
        let spans = resolve_spans(Some("agent:summary"), body).unwrap();
        assert_eq!(spans.len(), 2);
        assert!(spans[0].from_default);
        assert!(!spans[1].from_default);
        assert_eq!(display(spans[1].marker.as_ref()), "human:cody");
    }

    #[test]
    fn invalid_body_marker_errors() {
        let body = "<!-- prov robot -->\nx\n<!-- /prov -->";
        assert!(resolve_spans(None, body).is_err());
    }

    #[test]
    fn lenient_resolution_degrades_to_unknown() {
        let body = "<!-- prov robot -->\nx\n<!-- /prov -->";
        let spans = resolve_spans_lenient(None, body);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].marker, None);
        assert_eq!(spans[0].text, body);
    }

    #[test]
    fn filter_tokens_match() {
        let human = parse_spec("human:cody").unwrap();
        let agent = parse_spec("agent:inference").unwrap();
        let reviewed = parse_spec("agent:summary reviewed=2026-08-14").unwrap();

        assert!(matches_token(Some(&human), "human"));
        assert!(matches_token(Some(&human), "human:cody"));
        assert!(!matches_token(Some(&human), "human:dana"));
        assert!(matches_token(Some(&agent), "agent:inference"));
        assert!(!matches_token(Some(&agent), "reviewed"));
        assert!(matches_token(Some(&reviewed), "reviewed"));
        assert!(matches_token(None, "unknown"));
        assert!(!matches_token(None, "human"));
    }

    #[test]
    fn filter_token_validation() {
        for token in ["unknown", "reviewed", "human", "human:cody", "agent:index"] {
            assert!(
                validate_filter_token(token).is_ok(),
                "{token} must be valid"
            );
        }
        for token in ["robot", "agent:guess", "agent reviewed=1"] {
            assert!(
                validate_filter_token(token).is_err(),
                "{token} must be invalid"
            );
        }
    }

    #[test]
    fn empty_body_notes_match_through_the_default() {
        assert!(note_matches_tokens(Some("human:cody"), "", &["human"]));
        assert!(!note_matches_tokens(Some("human:cody"), "", &["agent"]));
        assert!(note_matches_tokens(None, "", &["unknown"]));
        assert!(note_has_unreviewed(Some("agent:summary"), ""));
        assert!(!note_has_unreviewed(Some("human"), ""));
        assert!(!note_has_unreviewed(None, ""));
    }

    #[test]
    fn mdstore_errors_are_not_double_prefixed() {
        let err = parse_spec("").unwrap_err().to_string();
        assert_eq!(err.matches("invalid provenance").count(), 1, "{err}");
    }

    #[test]
    fn citation_refs_collect_span_and_default_sources() {
        let body = "<!-- prov citation:a3f2 -->\n> q\n<!-- /prov -->\n<!-- prov citation src=https://x -->\n> r\n<!-- /prov -->";
        assert_eq!(citation_refs(None, body), vec!["a3f2".to_string()]);
        assert_eq!(citation_refs(Some("citation:b7c1"), "quoted text"), vec!["b7c1".to_string()]);
        assert!(citation_refs(Some("agent:summary"), "text").is_empty());
    }

    #[test]
    fn unreviewed_agent_detection() {
        let agent = parse_spec("agent:inference").unwrap();
        let reviewed = parse_spec("agent:inference reviewed=2026-08-14").unwrap();
        let human = parse_spec("human").unwrap();
        assert!(is_unreviewed_agent(Some(&agent)));
        assert!(!is_unreviewed_agent(Some(&reviewed)));
        assert!(!is_unreviewed_agent(Some(&human)));
        assert!(!is_unreviewed_agent(None));
    }
}
