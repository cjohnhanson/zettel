use std::fmt;
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The status of a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Draft,
    Permanent,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => f.write_str("draft"),
            Self::Permanent => f.write_str("permanent"),
        }
    }
}

impl FromStr for Status {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "permanent" => Ok(Self::Permanent),
            _ => Err(crate::error::Error::InvalidStatus { status: s.into() }),
        }
    }
}

impl Serialize for Status {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// The frontmatter of a note.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct NoteFrontmatter {
    pub title: String,
    #[serde(default)]
    pub status: Status,
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
        status: Status::Draft,
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
        status: fm.status,
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
    // serialize only fails when the frontmatter cannot be represented as
    // YAML, which a struct of strings and lists cannot.
    mdstore::document::serialize(&doc).unwrap_or_default()
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
            status: Status::Permanent,
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
        assert_eq!(back.status, Status::Permanent);
    }

    #[test]
    fn unknown_frontmatter_keys_survive_an_edit() {
        let source = "---\ntitle: t\nstatus: draft\ntags: []\nlinks: []\ncreated: 2020-01-01T00:00:00Z\nupdated: 2020-01-01T00:00:00Z\nauthor: someone\nproject: alpha\n---\n\nbody\n";
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
}
