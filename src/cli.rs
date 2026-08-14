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
    /// The root directory of the repository. The default is the current directory.
    #[arg(long, global = true, default_value = ".")]
    pub root: Utf8PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Initialize Zettel in the current directory
    Init,

    /// Write the man pages into a directory (the package build uses this)
    #[command(hide = true)]
    GenMan {
        /// Output directory for the section-1 pages
        dir: std::path::PathBuf,
    },

    /// Print a shell completion script (the package build uses this)
    #[command(hide = true)]
    GenCompletions {
        /// Target shell
        shell: clap_complete::Shell,
    },

    /// Manage the notes. The variant is boxed because NoteCommand is
    /// far larger than the other variants, and clippy flags the size
    /// difference.
    #[command(subcommand)]
    Note(Box<NoteCommand>),

    /// Show the notes that link to a note
    Backlinks(BacklinksArgs),

    /// Show the notes with no links
    Orphans(OrphansArgs),

    /// Search the notes with a regex pattern
    Search(SearchArgs),

    /// Show the full content of the matching notes
    Read(ReadArgs),

    /// Show a note and the linked notes within N hops
    Context(ContextArgs),

    /// Show the knowledge base statistics
    Stats(StatsArgs),

    /// Show the declared stores and what this vantage can see
    Store(StoreArgs),

    /// Check the notes for broken links and invalid provenance
    Check,

    /// Convert pre-provenance notes (status keys) to the provenance model
    Migrate,

    /// Read the bundled documentation
    Docs(DocsArgs),
}

#[derive(clap::Args)]
pub struct DocsArgs {
    /// The topic slug to show, or "search" to search the docs
    pub topic: Option<String>,

    /// The search query. Use it when the topic is "search".
    pub query: Option<String>,
}

#[derive(Parser)]
pub enum NoteCommand {
    /// Create a note
    Create(NoteCreateArgs),

    /// List the notes
    List(NoteListArgs),

    /// Show a note
    Show(NoteShowArgs),

    /// Edit a note
    Edit(NoteEditArgs),

    /// Delete a note
    Delete(NoteDeleteArgs),

    /// List the provenance spans of a note, or approve agent spans
    Review(NoteReviewArgs),
}

impl NoteCommand {
    /// The note a mutating subcommand targets, if it mutates one.
    fn target_id(&self) -> Option<&str> {
        match self {
            NoteCommand::Edit(a) => Some(&a.id),
            NoteCommand::Delete(a) => Some(&a.id),
            // Review only mutates with --approve; the listing form is a
            // read and may name a dependency's note.
            NoteCommand::Review(a) if a.approve.is_some() => Some(&a.id),
            _ => None,
        }
    }
}

#[derive(Parser)]
pub struct NoteCreateArgs {
    /// The note title
    pub title: String,

    /// The tags; separate them with commas
    #[arg(short, long = "tag")]
    pub tag: Option<String>,

    /// The link IDs; separate them with commas
    #[arg(short, long)]
    pub links: Option<String>,

    /// The note body text
    #[arg(short, long)]
    pub body: Option<String>,

    /// The default provenance: origin[:qualifier], for example human:cody,
    /// agent:summary, or citation:ab12. Omitted means unknown.
    #[arg(short, long)]
    pub provenance: Option<String>,
}

#[derive(Parser)]
pub struct NoteListArgs {
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Filter by provenance tokens, separated with commas: human, agent,
    /// agent:inference, citation, reviewed, unknown. A note matches when
    /// any of its spans matches any token.
    #[arg(short, long)]
    pub provenance: Option<String>,

    /// Keep only the notes with at least one unreviewed agent span
    #[arg(long)]
    pub unreviewed: bool,

    /// Filter by selector (namespace:value). Repeat the option to add selectors.
    #[arg(long = "where")]
    pub r#where: Vec<String>,

    /// The output format (text or json)
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
    /// The note ID
    pub id: String,

    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Print one field value
    #[arg(long)]
    pub field: Option<String>,
}

#[derive(Parser)]
pub struct NoteEditArgs {
    /// The note ID
    pub id: String,

    /// The new title
    #[arg(long)]
    pub title: Option<String>,

    /// The new default provenance: origin[:qualifier]
    #[arg(short, long)]
    pub provenance: Option<String>,

    /// Replace all tags
    #[arg(short, long = "tag")]
    pub tag: Option<String>,

    /// Add one tag and keep the existing tags
    #[arg(long)]
    pub add_tag: Option<String>,

    /// Remove one tag and keep the other tags
    #[arg(long)]
    pub remove_tag: Option<String>,

    /// Replace all links
    #[arg(short, long)]
    pub links: Option<String>,

    /// Add one link and keep the existing links
    #[arg(long)]
    pub add_link: Option<String>,

    /// Remove one link and keep the other links
    #[arg(long)]
    pub remove_link: Option<String>,

    /// Replace the whole body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Append text to the body
    #[arg(long)]
    pub append: Option<String>,
}

#[derive(Parser)]
pub struct NoteDeleteArgs {
    /// The note ID
    pub id: String,
}

#[derive(Parser)]
pub struct NoteReviewArgs {
    /// The note ID
    pub id: String,

    /// Approve spans: "all" for every unreviewed agent span, or 1-based
    /// span numbers separated with commas. Only a human runs this.
    #[arg(long)]
    pub approve: Option<String>,

    /// The reviewer name to record with the approval
    #[arg(long)]
    pub reviewer: Option<String>,
}

#[derive(Parser)]
pub struct BacklinksArgs {
    /// The note ID
    pub id: String,

    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct SearchArgs {
    /// The search pattern. Regular expressions work.
    pub pattern: String,

    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct ReadArgs {
    /// Filter by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Filter by provenance tokens, separated with commas. Only the
    /// matching spans print, and notes with no match are omitted.
    #[arg(short, long)]
    pub provenance: Option<String>,
}

#[derive(Parser)]
pub struct ContextArgs {
    /// The note ID to start from
    pub id: String,

    /// The maximum link depth to traverse. The default is 2.
    #[arg(short, long, default_value = "2")]
    pub depth: usize,

    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct OrphansArgs {
    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub command: StoreCommand,
}

#[derive(Parser)]
pub enum StoreCommand {
    /// List the stores this vantage can see
    List(StoreListArgs),
}

#[derive(Parser)]
pub struct StoreListArgs {
    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Parser)]
pub struct StatsArgs {
    /// The output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

/// Run Zettel with the given arguments.
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

        Command::GenMan { dir } => {
            use clap::CommandFactory as _;
            std::fs::create_dir_all(&dir)?;
            crate::mangen::write_man_pages(&Args::command(), &dir)?;
            Ok(())
        }

        Command::GenCompletions { shell } => {
            use clap::CommandFactory as _;
            clap_complete::generate(
                shell,
                &mut Args::command(),
                "zettel",
                &mut std::io::stdout(),
            );
            Ok(())
        }

        Command::Note(cmd) => {
            let repo = Repo::open(&root)?;
            // Dependency stores are read-only through the closure: a
            // write must run from the store that owns the note.
            if let Some(id) = cmd.target_id() {
                let ws = crate::workspace::Workspace::open(&root)?;
                if !ws.is_single_store()
                    && let Ok(view) = ws.find(id)
                {
                    ws.ensure_writable(&view, id)?;
                }
            }
            match *cmd {
                NoteCommand::Create(a) => {
                    let id = repo.create_note(
                        &a.title,
                        CreateNoteOptions {
                            tags: a.tag,
                            links: a.links,
                            body: a.body,
                            provenance: a.provenance,
                        },
                    )?;
                    println!("{id}");
                    Ok(())
                }
                NoteCommand::List(a) => {
                    let selectors: Vec<crate::Selector> = a
                        .r#where
                        .iter()
                        .map(|s| crate::selector::parse_selector(s))
                        .collect::<crate::Result<Vec<_>>>()?;
                    let mut notes = repo.list_notes(&ListNotesFilter {
                        tag: a.tag.as_deref(),
                        provenance: a.provenance.as_deref(),
                        unreviewed: a.unreviewed,
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
                    // Reads reach the whole closure, so an ID printed by
                    // any command can be pasted back into show.
                    let ws = crate::workspace::Workspace::open(&root)?;
                    let view = ws.find(&a.id)?;
                    if let Some(field) = &a.field {
                        print_note_field(view.note, field)?;
                    } else {
                        match a.format {
                            OutputFormat::Json => {
                                print_note_json_qualified(view.note, &view.qualified)
                            }
                            OutputFormat::Text => print_note_show(view.note, &view.qualified),
                        }
                    }
                    Ok(())
                }
                NoteCommand::Edit(a) => {
                    repo.edit_note(
                        &a.id,
                        EditNoteOptions {
                            title: a.title.as_deref(),
                            provenance: a.provenance.as_deref(),
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
                NoteCommand::Review(a) => {
                    match a.approve.as_deref() {
                        None => {
                            let (n, spans) = repo.note_spans(&a.id)?;
                            println!("{}  {}", n.id, n.frontmatter.title);
                            println!();
                            for s in &spans {
                                let source = if s.from_default { " (default)" } else { "" };
                                let excerpt = span_excerpt(&s.text);
                                println!(
                                    "  {}  {}{source}  {excerpt}",
                                    s.index,
                                    crate::provenance::display(s.marker.as_ref()),
                                );
                            }
                        }
                        Some("all") => {
                            let stamped = repo.approve_spans(&a.id, None, a.reviewer.as_deref())?;
                            println!("{stamped} span(s) approved");
                        }
                        Some(list) => {
                            let indices = list
                                .split(',')
                                .map(|s| {
                                    s.trim().parse::<usize>().map_err(|_| {
                                        crate::Error::InvalidProvenance(format!(
                                            "'{s}' is not a span number; use \"all\" or numbers separated with commas"
                                        ))
                                    })
                                })
                                .collect::<crate::Result<Vec<usize>>>()?;
                            let stamped =
                                repo.approve_spans(&a.id, Some(&indices), a.reviewer.as_deref())?;
                            println!("{stamped} span(s) approved");
                        }
                    }
                    Ok(())
                }
            }
        }

        Command::Backlinks(a) => {
            Repo::open(&root)?;
            let ws = crate::workspace::Workspace::open(&root)?;
            let target = ws.find(&a.id)?;
            let backlinks = ws.backlinks(target.id);
            match a.format {
                OutputFormat::Json => {
                    let arr: Vec<serde_json::Value> = backlinks
                        .iter()
                        .map(|v| {
                            serde_json::json!({
                                "id": v.qualified,
                                "title": v.note.frontmatter.title,
                                "store": ws.store_label(v.id),
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
                }
                OutputFormat::Text => {
                    for v in &backlinks {
                        println!("{}  {}", v.qualified, v.note.frontmatter.title);
                    }
                }
            }
            print_partial(&ws);
            Ok(())
        }

        Command::Orphans(a) => {
            Repo::open(&root)?;
            let ws = crate::workspace::Workspace::open(&root)?;
            let orphans = ws.orphans();
            match a.format {
                OutputFormat::Json => print_view_list_json(&ws, &orphans),
                OutputFormat::Text => print_view_list(&orphans),
            }
            print_partial(&ws);
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
                            v.as_object_mut().unwrap().insert(
                                "matched_fields".into(),
                                serde_json::json!(r.matched_fields),
                            );
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
            let notes = repo.list_notes(&ListNotesFilter {
                tag: a.tag.as_deref(),
                provenance: a.provenance.as_deref(),
                unreviewed: false,
            })?;
            let tokens: Option<Vec<&str>> = a
                .provenance
                .as_deref()
                .map(|t| t.split(',').map(str::trim).collect());
            for n in &notes {
                println!("--- {} ---", n.id);
                println!("title: {}", n.frontmatter.title);
                println!("provenance: {}", default_provenance_display(n));
                if !n.frontmatter.tags.is_empty() {
                    println!("tags: {}", n.frontmatter.tags.join(", "));
                }
                if !n.frontmatter.links.is_empty() {
                    println!("links: {}", n.frontmatter.links.join(", "));
                }
                println!();
                match &tokens {
                    // No filter: the whole body, markers and all.
                    None => {
                        if !n.body.is_empty() {
                            println!("{}", n.body);
                            println!();
                        }
                    }
                    // Filtered: only the matching spans, each labeled.
                    Some(tokens) => {
                        let spans = crate::provenance::resolve_spans_lenient(
                            n.frontmatter.provenance.as_deref(),
                            &n.body,
                        );
                        for s in spans
                            .iter()
                            .filter(|s| crate::provenance::matches_any(s.marker.as_ref(), tokens))
                        {
                            println!("[{}]", crate::provenance::display(s.marker.as_ref()));
                            if !s.text.is_empty() {
                                println!("{}", s.text);
                            }
                            println!();
                        }
                    }
                }
            }
            Ok(())
        }

        Command::Context(a) => {
            Repo::open(&root)?;
            let ws = crate::workspace::Workspace::open(&root)?;
            let start = ws.find(&a.id)?;
            let views = ws.neighborhood(start.id, a.depth);
            match a.format {
                OutputFormat::Json => print_view_list_json(&ws, &views),
                OutputFormat::Text => {
                    for (i, v) in views.iter().enumerate() {
                        if i > 0 {
                            println!();
                        }
                        let n = v.note;
                        // Foreign notes print store-qualified: an agent
                        // reading this stream must be able to tell a
                        // dependency's text from the vantage's own.
                        println!("--- {} ---", v.qualified);
                        println!("title: {}", n.frontmatter.title);
                        println!("provenance: {}", default_provenance_display(n));
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
            print_partial(&ws);
            Ok(())
        }

        Command::Store(a) => {
            Repo::open(&root)?;
            let ws = crate::workspace::Workspace::open(&root)?;
            match a.command {
                StoreCommand::List(args) => {
                    let members = ws.store_members();
                    match args.format {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string_pretty(&members).unwrap());
                        }
                        OutputFormat::Text => {
                            for m in &members {
                                let label = if m.alias.is_empty() {
                                    "(this store)".to_string()
                                } else {
                                    m.alias.clone()
                                };
                                let state = match &m.unavailable {
                                    Some(why) => format!("unavailable: {why}"),
                                    None => format!("{} note(s)", m.notes),
                                };
                                println!("{label}  {}  {state}", m.source);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        Command::Check => {
            let repo = Repo::open(&root)?;
            let mut broken = repo.check()?;
            let ws = crate::workspace::Workspace::open(&root)?;
            broken.extend(store_findings(&ws, &root));
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

        Command::Migrate => {
            let repo = Repo::open(&root)?;
            let changes = repo.migrate()?;
            if changes.is_empty() {
                println!("nothing to migrate");
            } else {
                for (id, action) in &changes {
                    println!("{id}  {action}");
                }
                println!("{} note(s) migrated", changes.len());
            }
            Ok(())
        }

        Command::Docs(args) => match args.topic.as_deref() {
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
        },

        Command::Stats(a) => {
            let repo = Repo::open(&root)?;
            let stats = repo.stats()?;
            match a.format {
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "total": stats.total,
                        "spans": {
                            "human": stats.span_counts.human,
                            "agent": stats.span_counts.agent,
                            "citation": stats.span_counts.citation,
                            "unknown": stats.span_counts.unknown,
                            "unreviewed_agent": stats.span_counts.unreviewed_agent,
                        },
                        "orphans": stats.orphan_count,
                        "tags": stats.tag_counts.iter().map(|(t, c)| serde_json::json!({"tag": t, "count": c})).collect::<Vec<_>>(),
                        "most_connected": stats.most_connected.iter().map(|(id, title, c)| serde_json::json!({"id": id, "title": title, "backlinks": c})).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                OutputFormat::Text => {
                    println!("{} notes", stats.total);
                    let c = &stats.span_counts;
                    println!(
                        "spans: {} human, {} agent ({} unreviewed), {} citation, {} unknown",
                        c.human, c.agent, c.unreviewed_agent, c.citation, c.unknown
                    );
                    println!("{} orphans", stats.orphan_count);
                    if !stats.tag_counts.is_empty() {
                        println!();
                        println!("Tags:");
                        for (tag, count) in &stats.tag_counts {
                            println!("  {tag}: {count}");
                        }
                    }
                    if !stats.most_connected.is_empty()
                        && stats.most_connected.iter().any(|(_, _, c)| *c > 0)
                    {
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
        println!(
            "{}  {}{tags}  {}",
            n.id,
            default_provenance_display(n),
            n.frontmatter.title
        );
    }
}

/// The note's default provenance spec for display, or `unknown`.
fn default_provenance_display(n: &Note) -> &str {
    n.frontmatter.provenance.as_deref().unwrap_or("unknown")
}

/// Print the notes of a closure, each with the ID it answers to from
/// this vantage.
fn print_view_list(views: &[crate::workspace::View<'_>]) {
    for v in views {
        let tags = if v.note.frontmatter.tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", v.note.frontmatter.tags.join(", "))
        };
        println!(
            "{}  {}{tags}  {}",
            v.qualified,
            default_provenance_display(v.note),
            v.note.frontmatter.title
        );
    }
}

fn print_view_list_json(ws: &crate::workspace::Workspace, views: &[crate::workspace::View<'_>]) {
    let arr: Vec<serde_json::Value> = views
        .iter()
        .map(|v| {
            let mut json = note_to_json(v.note);
            let obj = json.as_object_mut().unwrap();
            obj.insert("id".into(), serde_json::json!(v.qualified));
            obj.insert("store".into(), serde_json::json!(ws.store_label(v.id)));
            json
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
}

/// A graph answer computed over an incomplete closure is not wrong-but-
/// close; it is wrong. Say so rather than presenting it as whole.
fn print_partial(ws: &crate::workspace::Workspace) {
    let missing = ws.missing();
    if !missing.is_empty() {
        eprintln!("partial — unreachable store(s): {}", missing.join(", "));
    }
}

/// Findings that only the store layer can see.
fn store_findings(
    ws: &crate::workspace::Workspace,
    root: &camino::Utf8Path,
) -> Vec<crate::BrokenLink> {
    let mut findings = Vec::new();

    // Every reference that named no note, local or cross-store. One
    // resolution path, so a link that works cannot be reported broken.
    for (view, target) in ws.dangling() {
        // Where the reference was written, so the reader knows what to
        // edit: a frontmatter links entry or a [[ref]] in the body.
        let location = if view.note.frontmatter.links.iter().any(|l| l == &target) {
            "frontmatter"
        } else {
            "body"
        };
        findings.push(crate::BrokenLink {
            source_id: view.qualified.clone(),
            source_title: view.note.frontmatter.title.clone(),
            target,
            location: location.to_string(),
        });
    }

    // Declarations other clones could not follow.
    for (alias, why) in ws.unshareable(root) {
        findings.push(crate::BrokenLink {
            source_id: "stores.yml".to_string(),
            source_title: "store declarations".to_string(),
            target: format!("{alias}: {why}"),
            location: "declaration".to_string(),
        });
    }

    // A store that is declared but not there.
    for alias in ws.missing() {
        findings.push(crate::BrokenLink {
            source_id: "stores.yml".to_string(),
            source_title: "store declarations".to_string(),
            target: alias,
            location: "unreachable store".to_string(),
        });
    }

    // A citation key that a newly declared alias now shadows: it used to
    // be an opaque external key and now reads as a store reference.
    for (note_id, source) in ws.shadowed_citations() {
        findings.push(crate::BrokenLink {
            source_id: note_id,
            source_title: "citation".to_string(),
            target: format!("'{source}' now reads as a store reference"),
            location: "shadowed citation".to_string(),
        });
    }

    // Files the guarded scan refused to read.
    for skipped in ws.skipped() {
        findings.push(crate::BrokenLink {
            source_id: "store".to_string(),
            source_title: "skipped file".to_string(),
            target: skipped.clone(),
            location: "scan".to_string(),
        });
    }

    findings
}

/// The first line of a span, shortened for the review listing.
fn span_excerpt(text: &str) -> String {
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut excerpt: String = first.trim().chars().take(60).collect();
    if first.trim().chars().count() > 60 {
        excerpt.push('…');
    }
    excerpt
}

fn print_note_list_json(notes: &[Note]) {
    let arr: Vec<serde_json::Value> = notes.iter().map(note_to_json).collect();
    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
}

fn note_to_json(n: &Note) -> serde_json::Value {
    serde_json::json!({
        "id": n.id,
        "title": n.frontmatter.title,
        "provenance": n.frontmatter.provenance,
        "tags": n.frontmatter.tags,
        "links": n.frontmatter.links,
        "body": n.body,
    })
}

/// Show a note under the ID it answers to from this vantage.
fn print_note_json_qualified(n: &Note, qualified: &str) {
    print_note_json_with_id(n, qualified)
}


fn print_note_json_with_id(n: &Note, qualified: &str) {
    // `show --format json` carries the resolved spans so an agent can
    // treat each piece of text by its provenance.
    let spans: Vec<serde_json::Value> =
        crate::provenance::resolve_spans_lenient(n.frontmatter.provenance.as_deref(), &n.body)
            .iter()
            .map(|s| {
                serde_json::json!({
                    "provenance": crate::provenance::display(s.marker.as_ref()),
                    "from_default": s.from_default,
                    "text": s.text,
                })
            })
            .collect();
    let mut json = note_to_json(n);
    let obj = json.as_object_mut().unwrap();
    obj.insert("id".into(), serde_json::json!(qualified));
    obj.insert("spans".into(), serde_json::Value::Array(spans));
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

fn print_note_show(n: &Note, qualified: &str) {
    println!("{qualified}");
    println!();
    println!("  {:<14}{}", "Title:", n.frontmatter.title);
    println!("  {:<14}{}", "Provenance:", default_provenance_display(n));
    if !n.frontmatter.tags.is_empty() {
        println!("  {:<14}{}", "Tags:", n.frontmatter.tags.join(", "));
    }
    if !n.frontmatter.links.is_empty() {
        println!("  {:<14}{}", "Links:", n.frontmatter.links.join(", "));
    }
    if !n.body.is_empty() {
        println!();
        println!("{}", n.body);
    }
}

fn print_note_field(n: &Note, field: &str) -> crate::Result<()> {
    let value = match field {
        "title" => Some(n.frontmatter.title.clone()),
        "provenance" => Some(default_provenance_display(n).to_string()),
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
