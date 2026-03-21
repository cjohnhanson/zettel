use camino::Utf8PathBuf;
use clap::Parser;

use crate::{CreateNoteOptions, EditNoteOptions, Note, Repo};

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
    Orphans,
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
    #[arg(short, long)]
    pub tags: Option<String>,

    /// Comma-separated link IDs
    #[arg(short, long)]
    pub links: Option<String>,

    /// Note body text (inline)
    #[arg(short, long)]
    pub body: Option<String>,
}

#[derive(Parser)]
pub struct NoteListArgs {
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,

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

    /// New tags (replaces existing)
    #[arg(short, long)]
    pub tags: Option<String>,

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
    #[arg(long)]
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
                    let id = repo.create_note(
                        &a.title,
                        CreateNoteOptions {
                            tags: a.tags,
                            links: a.links,
                            body: a.body,
                        },
                    )?;
                    println!("{id}");
                    Ok(())
                }
                NoteCommand::List(a) => {
                    let notes = repo.list_notes(a.tag.as_deref())?;
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
                            tags: a.tags.as_deref(),
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

        Command::Orphans => {
            let repo = Repo::open(&root)?;
            let orphans = repo.orphans()?;
            print_note_list(&orphans);
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
        println!("{}{tags}  {}", n.id, n.frontmatter.title);
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
