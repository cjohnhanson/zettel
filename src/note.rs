use chrono::Utc;
use serde::{Deserialize, Serialize};

/// The frontmatter of a note.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct NoteFrontmatter {
    pub title: String,
    /// The default provenance spec for unmarked body text:
    /// `origin[:qualifier] [key=value ...]`. A missing key means the
    /// provenance is unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Forward links to other notes. Each link is a full ID or a prefix.
    #[serde(default)]
    pub links: Vec<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    /// Frontmatter keys zettel does not model. They are kept so an edit
    /// never drops a key a user or another tool added.
    #[serde(flatten)]
    pub extra: serde_yml::Mapping,
}

/// A parsed note with its ID and content.
#[derive(Debug, Serialize)]
pub struct Note {
    pub id: String,
    pub frontmatter: NoteFrontmatter,
    pub body: String,
}

pub fn new_frontmatter(title: &str) -> NoteFrontmatter {
    let now = format!("{}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    NoteFrontmatter {
        title: title.into(),
        provenance: None,
        tags: vec![],
        links: vec![],
        created: Some(now.clone()),
        updated: Some(now),
        extra: serde_yml::Mapping::new(),
    }
}

pub fn update_timestamp(fm: &mut NoteFrontmatter) {
    fm.updated = Some(format!("{}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));
}

/// Parse the content of a note file into frontmatter and body.
pub fn parse_note(content: &str) -> crate::error::Result<(NoteFrontmatter, String)> {
    let doc = mdstore::document::parse::<NoteFrontmatter>(content).map_err(|e| match e {
        mdstore::Error::MissingFrontmatter | mdstore::Error::UnclosedFrontmatter => {
            crate::error::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
        mdstore::Error::Yaml(ye) => crate::error::Error::Yaml(ye),
        other => crate::error::Error::InvalidProvenance(other.to_string()),
    })?;
    Ok((doc.frontmatter, doc.body))
}

/// Serialize a note to frontmattered markdown.
///
/// serde_yml does the YAML, so a title, tag, or link with a comma, a
/// quote, a colon, or a backslash is escaped correctly. The old
/// hand-rolled writer produced a file that no longer parsed, and one
/// bad file broke repo-wide commands. Unknown frontmatter keys survive
/// through the `extra` map.
pub fn serialize_note(fm: &NoteFrontmatter, body: &str) -> String {
    let now = format!("{}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    let filled = NoteFrontmatter {
        title: fm.title.clone(),
        provenance: fm.provenance.clone(),
        tags: fm.tags.clone(),
        links: fm.links.clone(),
        created: Some(fm.created.clone().unwrap_or_else(|| now.clone())),
        updated: Some(fm.updated.clone().unwrap_or(now)),
        extra: fm.extra.clone(),
    };
    let doc = mdstore::document::Document {
        frontmatter: filled,
        body: body.to_string(),
    };
    mdstore::document::serialize(&doc).expect("frontmatter serialization must not fail")
}

#[cfg(test)]
mod serialize_tests {
    use super::*;

    fn round_trip(fm: &NoteFrontmatter, body: &str) -> (NoteFrontmatter, String) {
        let text = serialize_note(fm, body);
        parse_note(&text).expect("a serialized note must parse back")
    }

    #[test]
    fn metacharacters_in_fields_round_trip() {
        // Commas, brackets, quotes, colons, and a backslash used to break
        // the hand-rolled writer and produce an unparseable file.
        let fm = NoteFrontmatter {
            title: r#"a: "quoted", [bracket] and a \ backslash"#.to_string(),
            provenance: Some("human:cody".to_string()),
            tags: vec!["a, b".to_string(), "c: d".to_string(), "[e]".to_string()],
            links: vec!["ab12".to_string(), "x, y".to_string()],
            created: Some("2020-01-01T00:00:00Z".to_string()),
            updated: Some("2020-01-02T00:00:00Z".to_string()),
            extra: serde_yml::Mapping::new(),
        };
        let (back, _) = round_trip(&fm, "body text");
        assert_eq!(back.title, fm.title);
        assert_eq!(back.tags, fm.tags);
        assert_eq!(back.links, fm.links);
        assert_eq!(back.provenance.as_deref(), Some("human:cody"));
    }

    #[test]
    fn missing_provenance_stays_absent_after_an_edit() {
        let fm = new_frontmatter("t");
        let text = serialize_note(&fm, "body");
        assert!(
            !text.contains("provenance"),
            "no provenance key when unknown"
        );
        let (back, _) = parse_note(&text).unwrap();
        assert_eq!(back.provenance, None);
    }

    #[test]
    fn unknown_frontmatter_keys_survive_an_edit() {
        let source = "---\ntitle: t\ntags: []\nlinks: []\ncreated: 2020-01-01T00:00:00Z\nupdated: 2020-01-01T00:00:00Z\nauthor: someone\nproject: alpha\n---\n\nbody\n";
        let (mut fm, body) = parse_note(source).unwrap();
        assert!(fm.extra.contains_key("author"), "unknown key captured");
        update_timestamp(&mut fm);
        let (back, _) = round_trip(&fm, &body);
        assert_eq!(
            back.extra.get("author").and_then(|v| v.as_str()),
            Some("someone")
        );
        assert_eq!(
            back.extra.get("project").and_then(|v| v.as_str()),
            Some("alpha")
        );
    }

    #[test]
    fn pre_migration_status_key_survives_as_extra() {
        // A note from before the provenance change still parses, and its
        // status key is kept for `zettel migrate` to convert.
        let source = "---\ntitle: t\nstatus: permanent\ntags: []\nlinks: []\ncreated: 2020-01-01T00:00:00Z\nupdated: 2020-01-01T00:00:00Z\n---\n\nbody\n";
        let (fm, _) = parse_note(source).unwrap();
        assert_eq!(
            fm.extra.get("status").and_then(|v| v.as_str()),
            Some("permanent")
        );
        assert_eq!(fm.provenance, None);
    }

    #[test]
    fn body_markers_round_trip_verbatim() {
        let fm = new_frontmatter("t");
        let body = "before\n<!-- prov agent:inference -->\nguess\n<!-- /prov -->\nafter";
        let (_, back_body) = round_trip(&fm, body);
        assert_eq!(back_body, body);
    }
}
