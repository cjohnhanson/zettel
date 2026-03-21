use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Frontmatter for a zettelkasten note.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct NoteFrontmatter {
    pub title: String,
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
