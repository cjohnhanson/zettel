use std::fmt;
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Note maturity status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
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

impl Default for Status {
    fn default() -> Self {
        Self::Draft
    }
}

/// Frontmatter for a zettelkasten note.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct NoteFrontmatter {
    pub title: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Forward links to other notes by ID (prefix or full).
    #[serde(default)]
    pub links: Vec<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

/// A parsed zettel note with its ID and content.
#[derive(Debug, Serialize)]
pub struct Note {
    pub id: String,
    pub frontmatter: NoteFrontmatter,
    pub body: String,
}

pub fn new_frontmatter(title: &str) -> NoteFrontmatter {
    let now = format!("\"{}\"", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    NoteFrontmatter {
        title: title.into(),
        status: Status::Draft,
        tags: vec![],
        links: vec![],
        created: Some(now.clone()),
        updated: Some(now),
    }
}

pub fn update_timestamp(fm: &mut NoteFrontmatter) {
    fm.updated = Some(format!("\"{}\"", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));
}

/// Parse a note file's content into frontmatter and body.
pub fn parse_note(content: &str) -> crate::error::Result<(NoteFrontmatter, String)> {
    let doc = mdstore::document::parse::<NoteFrontmatter>(content).map_err(|e| match e {
        mdstore::Error::MissingFrontmatter | mdstore::Error::UnclosedFrontmatter => {
            crate::error::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
        mdstore::Error::Yaml(ye) => crate::error::Error::Yaml(ye),
    })?;
    Ok((doc.frontmatter, doc.body))
}

/// Serialize a note back to frontmattered markdown.
pub fn serialize_note(fm: &NoteFrontmatter, body: &str) -> String {
    let mut s = String::from("---\n");
    s.push_str(&format!("title: \"{}\"\n", fm.title.replace('"', "\\\"")));
    s.push_str(&format!("status: {}\n", fm.status));

    if fm.tags.is_empty() {
        s.push_str("tags: []\n");
    } else {
        s.push_str(&format!("tags: [{}]\n", fm.tags.join(", ")));
    }

    if fm.links.is_empty() {
        s.push_str("links: []\n");
    } else {
        s.push_str(&format!("links: [{}]\n", fm.links.join(", ")));
    }

    let created = fm
        .created
        .clone()
        .unwrap_or_else(|| format!("\"{}\"", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));
    let updated = fm
        .updated
        .clone()
        .unwrap_or_else(|| format!("\"{}\"", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));

    s.push_str(&format!("created: {created}\n"));
    s.push_str(&format!("updated: {updated}\n"));
    s.push_str("---\n");

    if !body.is_empty() {
        s.push('\n');
        s.push_str(body);
        s.push('\n');
    }

    s
}
