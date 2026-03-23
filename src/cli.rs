use camino::Utf8PathBuf;
use clap::Parser;

use crate::{CreateNoteOptions, EditNoteOptions, ListNotesFilter, Note, Repo};

#[derive(Parser)]
#[command(
    name = "zettel",
    version,
    about = "Zettelkasten note management on frontmattered markdown",
    max_term_width = 98
)]
pub struct Args {
    /// Root directory of the repository (default: current directory)
    #[arg(long, global = true, default_value = ".")]
    pub root: Utf8PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Initialize zettel in the current directory
    Init,

    /// Manage notes
    #[command(subcommand)]
    Note(NoteCommand),

    /// Show backlinks to a note
    Backlinks(BacklinksArgs),

    /// Show orphaned notes (no links in or out)
    Orphans(OrphansArgs),

    /// Search notes by regex pattern
    Search(SearchArgs),

    /// Read full content of matching notes
    Read(ReadArgs),

    /// Show a note and its neighborhood (linked notes within N hops)
    Context(ContextArgs),

    /// Show knowledge base statistics
    Stats(StatsArgs),

    /// Check for broken links and other issues
    Check,

    /// Browse bundled documentation
    Docs(DocsArgs),
}

#[derive(clap::Args)]
pub struct DocsArgs {
    /// Topic slug to display, or "search" to search
    pub topic: Option<String>,

    /// Search query (when topic is "search")
    pub query: Option<String>,
}

#[derive(Parser)]
pub enum NoteCommand {
    /// Create a new note
    Create(NoteCreateArgs),

    /// List notes
    List(NoteListArgs),

    /// Show a note
    Show(NoteShowArgs),

    /// Edit a note
    Edit(NoteEditArgs),

    /// Delete a note
    Delete(NoteDeleteArgs),
}

#[derive(Parser)]
pub struct NoteCreateArgs {
    /// Note title
    pub title: String,

    /// Comma-separated tags
    #[arg(short, long = "tag")]
    pub tag: Option<String>,

    /// Comma-separated link IDs
    #[arg(short, long)]
    pub links: Option<String>,

    /// Note body text (inline)
    #[arg(short, long)]
    pub body: Option<String>,

    /// Initial status (default: draft)
    #[arg(short, long, default_value = "draft")]
    pub status: Option<String>,
}

#[derive(Parser)]
pub struct NoteListArgs {
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Filter by status (draft or permanent)
    #[arg(short, long)]
    pub status: Option<String>,

    /// Filter by selector (namespace:value, repeatable, ANDs together)
    #[arg(long = "where")]
    pub r#where: Vec<String>,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser)]
pub struct NoteShowArgs {
    /// Note ID
    pub id: String,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Extract a single field value
    #[arg(long)]
    pub field: Option<String>,
}

#[derive(Parser)]
pub struct NoteEditArgs {
    /// Note ID
    pub id: String,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New status (draft or permanent)
    #[arg(short, long)]
    pub status: Option<String>,

    /// New tags (replaces existing)
    #[arg(short, long = "tag")]
    pub tag: Option<String>,

    /// Add a tag
    #[arg(long)]
    pub add_tag: Option<String>,

    /// Remove a tag
    #[arg(long)]
    pub remove_tag: Option<String>,

    /// New links (replaces existing)
    #[arg(short, long)]
    pub links: Option<String>,

    /// Add a link
    #[arg(long)]
    pub add_link: Option<String>,

    /// Remove a link
    #[arg(long)]
    pub remove_link: Option<String>,

    /// Replace the entire body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Append text to the body
    #[arg(long)]
    pub append: Option<String>,
}

#[derive(Parser)]
pub struct NoteDeleteArgs {
    /// Note ID
    pub id: String,
}

#[derive(Parser)]
pub struct BacklinksArgs {
    /// Note ID to find backlinks for
    pub id: String,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct SearchArgs {
    /// Search pattern (regex supported)
    pub pattern: String,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct ReadArgs {
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Filter by status (draft or permanent)
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(Parser)]
pub struct ContextArgs {
    /// Note ID to explore from
    pub id: String,

    /// Maximum link depth to traverse (default: 2)
    #[arg(short, long, default_value = "2")]
    pub depth: usize,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct OrphansArgs {
    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct StatsArgs {
    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

/// Run zettel with the given arguments.
pub fn run(args: Args) -> crate::Result<()> {
    let root = if args.root.is_relative() {
        let cwd = std::env::current_dir()?;
        Utf8PathBuf::try_from(cwd)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            .join(&args.root)
    } else {
        args.root.clone()
    };

    match args.command {
        Command::Init => Repo::init(&root),

        Command::Note(cmd) => {
            let repo = Repo::open(&root)?;
            match cmd {
                NoteCommand::Create(a) => {
                    let status = a
                        .status
                        .map(|s| s.parse::<crate::note::Status>())
                        .transpose()?;
                    let id = repo.create_note(
                        &a.title,
                        CreateNoteOptions {
                            tags: a.tag,
                            links: a.links,
                            body: a.body,
                            status,
                        },
                    )?;
                    println!("{id}");
                    Ok(())
                }
                NoteCommand::List(a) => {
                    let status = a
                        .status
                        .map(|s| s.parse::<crate::note::Status>())
                        .transpose()?;
                    let selectors: Vec<crate::Selector> = a
                        .r#where
                        .iter()
                        .filter_map(|s| crate::Selector::parse(s))
                        .collect();
                    let mut notes = repo.list_notes(&ListNotesFilter {
                        tag: a.tag.as_deref(),
                        status,
                    })?;
                    if !selectors.is_empty() {
                        notes.retain(|n| crate::selector::matches_all(&selectors, n));
                    }
                    match a.format {
                        OutputFormat::Json => print_note_list_json(&notes),
                        OutputFormat::Text => print_note_list(&notes),
                    }
                    Ok(())
                }
                NoteCommand::Show(a) => {
                    let n = repo.find_note(&a.id)?;
                    if let Some(field) = &a.field {
                        print_note_field(&n, field)?;
                    } else {
                        match a.format {
                            OutputFormat::Json => print_note_json(&n),
                            OutputFormat::Text => print_note_show(&n),
                        }
                    }
                    Ok(())
                }
                NoteCommand::Edit(a) => {
                    repo.edit_note(
                        &a.id,
                        EditNoteOptions {
                            title: a.title.as_deref(),
                            status: a.status.as_deref(),
                            tags: a.tag.as_deref(),
                            add_tag: a.add_tag.as_deref(),
                            remove_tag: a.remove_tag.as_deref(),
                            links: a.links.as_deref(),
                            add_link: a.add_link.as_deref(),
                            remove_link: a.remove_link.as_deref(),
                            body: a.body.as_deref(),
                            append: a.append.as_deref(),
                        },
                    )?;
                    Ok(())
                }
                NoteCommand::Delete(a) => {
                    repo.delete_note(&a.id)?;
                    Ok(())
                }
            }
        }

        Command::Backlinks(a) => {
            let repo = Repo::open(&root)?;
            let backlinks = repo.backlinks(&a.id)?;
            match a.format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&backlinks).unwrap();
                    println!("{json}");
                }
                OutputFormat::Text => {
                    for bl in &backlinks {
                        println!("{}  {}", bl.id, bl.title);
                    }
                }
            }
            Ok(())
        }

        Command::Orphans(a) => {
            let repo = Repo::open(&root)?;
            let orphans = repo.orphans()?;
            match a.format {
                OutputFormat::Json => print_note_list_json(&orphans),
                OutputFormat::Text => print_note_list(&orphans),
            }
            Ok(())
        }

        Command::Search(a) => {
            let repo = Repo::open(&root)?;
            let results = repo.search(&a.pattern)?;
            match a.format {
                OutputFormat::Json => {
                    let arr: Vec<serde_json::Value> = results
                        .iter()
                        .map(|r| {
                            let mut v = note_to_json(&r.note);
                            v.as_object_mut()
                                .unwrap()
                                .insert("matched_fields".into(), serde_json::json!(r.matched_fields));
                            v
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
                }
                OutputFormat::Text => {
                    for r in &results {
                        println!(
                            "{}  {}  ({})",
                            r.note.id,
                            r.note.frontmatter.title,
                            r.matched_fields.join(", ")
                        );
                    }
                }
            }
            Ok(())
        }

        Command::Read(a) => {
            let repo = Repo::open(&root)?;
            let status = a
                .status
                .map(|s| s.parse::<crate::note::Status>())
                .transpose()?;
            let notes = repo.list_notes(&ListNotesFilter {
                tag: a.tag.as_deref(),
                status,
            })?;
            for n in &notes {
                println!("--- {} ---", n.id);
                println!("title: {}", n.frontmatter.title);
                println!("status: {}", n.frontmatter.status);
                if !n.frontmatter.tags.is_empty() {
                    println!("tags: {}", n.frontmatter.tags.join(", "));
                }
                if !n.frontmatter.links.is_empty() {
                    println!("links: {}", n.frontmatter.links.join(", "));
                }
                println!();
                if !n.body.is_empty() {
                    println!("{}", n.body);
                    println!();
                }
            }
            Ok(())
        }

        Command::Context(a) => {
            let repo = Repo::open(&root)?;
            let notes = repo.context(&a.id, a.depth)?;
            match a.format {
                OutputFormat::Json => print_note_list_json(&notes),
                OutputFormat::Text => {
                    for (i, n) in notes.iter().enumerate() {
                        if i > 0 {
                            println!();
                        }
                        println!("--- {} ---", n.id);
                        println!("title: {}", n.frontmatter.title);
                        println!("status: {}", n.frontmatter.status);
                        if !n.frontmatter.tags.is_empty() {
                            println!("tags: {}", n.frontmatter.tags.join(", "));
                        }
                        if !n.frontmatter.links.is_empty() {
                            println!("links: {}", n.frontmatter.links.join(", "));
                        }
                        if !n.body.is_empty() {
                            println!();
                            println!("{}", n.body);
                        }
                    }
                }
            }
            Ok(())
        }

        Command::Check => {
            let repo = Repo::open(&root)?;
            let broken = repo.check()?;
            if broken.is_empty() {
                println!("no broken links");
            } else {
                println!("{} broken link(s):", broken.len());
                for bl in &broken {
                    println!(
                        "  {} ({}) → {} [{}]",
                        bl.source_id, bl.source_title, bl.target, bl.location
                    );
                }
                std::process::exit(1);
            }
            Ok(())
        }

        Command::Docs(args) => {
            match args.topic.as_deref() {
                None | Some("list") => {
                    crate::docs::list();
                    Ok(())
                }
                Some("search") => {
                    let query = args.query.as_deref().unwrap_or("");
                    crate::docs::search(query);
                    Ok(())
                }
                Some(slug) => {
                    if crate::docs::show(slug) {
                        Ok(())
                    } else {
                        eprintln!("unknown doc: '{slug}'");
                        eprintln!();
                        eprintln!("available docs:");
                        crate::docs::list();
                        Err(crate::Error::Io(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("doc '{slug}' not found"),
                        )))
                    }
                }
            }
        }

        Command::Stats(a) => {
            let repo = Repo::open(&root)?;
            let stats = repo.stats()?;
            match a.format {
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "total": stats.total,
                        "draft": stats.draft_count,
                        "permanent": stats.permanent_count,
                        "orphans": stats.orphan_count,
                        "tags": stats.tag_counts.iter().map(|(t, c)| serde_json::json!({"tag": t, "count": c})).collect::<Vec<_>>(),
                        "most_connected": stats.most_connected.iter().map(|(id, title, c)| serde_json::json!({"id": id, "title": title, "backlinks": c})).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                OutputFormat::Text => {
                    println!("{} notes ({} draft, {} permanent)", stats.total, stats.draft_count, stats.permanent_count);
                    println!("{} orphans", stats.orphan_count);
                    if !stats.tag_counts.is_empty() {
                        println!();
                        println!("Tags:");
                        for (tag, count) in &stats.tag_counts {
                            println!("  {tag}: {count}");
                        }
                    }
                    if !stats.most_connected.is_empty() && stats.most_connected.iter().any(|(_, _, c)| *c > 0) {
                        println!();
                        println!("Most connected:");
                        for (id, title, count) in &stats.most_connected {
                            if *count > 0 {
                                println!("  {id}  {title}  ({count} backlinks)");
                            }
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

fn print_note_list(notes: &[Note]) {
    for n in notes {
        let tags = if n.frontmatter.tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", n.frontmatter.tags.join(", "))
        };
        println!("{}  {}{tags}  {}", n.id, n.frontmatter.status, n.frontmatter.title);
    }
}

fn print_note_list_json(notes: &[Note]) {
    let arr: Vec<serde_json::Value> = notes.iter().map(note_to_json).collect();
    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
}

fn note_to_json(n: &Note) -> serde_json::Value {
    serde_json::json!({
        "id": n.id,
        "title": n.frontmatter.title,
        "status": n.frontmatter.status.to_string(),
        "tags": n.frontmatter.tags,
        "links": n.frontmatter.links,
        "body": n.body,
    })
}

fn print_note_json(n: &Note) {
    println!("{}", serde_json::to_string_pretty(&note_to_json(n)).unwrap());
}

fn print_note_show(n: &Note) {
    println!("{}", n.id);
    println!();
    println!("  {:<10}{}", "Title:", n.frontmatter.title);
    println!("  {:<10}{}", "Status:", n.frontmatter.status);
    if !n.frontmatter.tags.is_empty() {
        println!("  {:<10}{}", "Tags:", n.frontmatter.tags.join(", "));
    }
    if !n.frontmatter.links.is_empty() {
        println!("  {:<10}{}", "Links:", n.frontmatter.links.join(", "));
    }
    if !n.body.is_empty() {
        println!();
        println!("{}", n.body);
    }
}

fn print_note_field(n: &Note, field: &str) -> crate::Result<()> {
    let value = match field {
        "title" => Some(n.frontmatter.title.clone()),
        "status" => Some(n.frontmatter.status.to_string()),
        "tags" => Some(n.frontmatter.tags.join(", ")),
        "links" => Some(n.frontmatter.links.join(", ")),
        "body" => Some(n.body.clone()),
        "id" => Some(n.id.clone()),
        _ => return Err(crate::error::Error::UnknownField(field.into())),
    };
    if let Some(v) = value {
        println!("{v}");
    }
    Ok(())
}
