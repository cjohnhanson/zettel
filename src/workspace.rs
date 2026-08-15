//! The vantage a command runs from: this store plus the stores it
//! declares.
//!
//! Every command runs *from* one store. What it can see is that store's
//! dependency closure, so a repository store never sees the private user
//! store that annotates it. The repository does not declare the user
//! store. It cannot: the target does not exist for the other users who
//! clone the repository.

use camino::{Utf8Path, Utf8PathBuf};
use mdstore::snapshot::{DocId, DocumentSource, Entry, Snapshot};
use mdstore::registry::{Registry, RegistryLocator};
use mdstore::store::{FetchingLocator, LocalPaths, StoreContent, StoreGraph, StoreRef};

use crate::config::ZettelConfig;
use crate::error::{Error, Result};
use crate::note::{self, Note};

/// Reads zettel notes out of a store directory.
pub struct NoteSource;

impl DocumentSource for NoteSource {
    type Doc = Note;

    fn load(
        &self,
        content: &StoreContent,
        skipped: &mut Vec<String>,
    ) -> mdstore::Result<Vec<Entry<Note>>> {
        // A dependency's own config chooses its note directory, so the
        // value is guarded: it may not be absolute or climb out of the
        // store.
        let dir_name = if content.exists("zettel.yml") {
            let text = content.read("zettel.yml")?;
            yaml_serde::from_str::<ZettelConfig>(&text)
                .map(|c| c.zettel_dir)
                .unwrap_or_else(|_| ".zettel".to_string())
        } else {
            ".zettel".to_string()
        };
        // Reject a directory value that is absolute or climbs out.
        if let Some(dir) = content.dir() {
            mdstore::store::document_dir(dir, &dir_name)?;
        }

        let scan = content.scan(&dir_name)?;
        for (path, why) in scan.skipped {
            skipped.push(format!("{} ({why})", path.display()));
        }

        let mut notes = Vec::new();
        for entry in scan.entries {
            let text = content.read(&entry.path.to_string_lossy())?;
            // One unparseable note never takes down a store.
            match note::parse_note(&text) {
                Ok((frontmatter, body)) => notes.push(Entry {
                    id: entry.stem.clone(),
                    doc: Note {
                        id: entry.stem,
                        frontmatter,
                        body,
                    },
                }),
                Err(e) => skipped.push(format!("{} ({e})", entry.path.display())),
            }
        }
        Ok(notes)
    }

    /// Everything a note points at: frontmatter links, `[[ref]]` body
    /// references, and citation sources. Each is parsed against the
    /// aliases declared by the store holding this note.
    fn references(&self, doc: &Note, member: usize, graph: &StoreGraph) -> Vec<StoreRef> {
        let aliases = graph.config(member).aliases();
        let mut refs = Vec::new();
        let mut push = |text: &str| {
            let r = StoreRef::parse(text, &aliases);
            if !refs.contains(&r) {
                refs.push(r);
            }
        };
        for link in &doc.frontmatter.links {
            push(link);
        }
        for body_ref in crate::repo::extract_body_refs(&doc.body) {
            push(&body_ref);
        }
        refs
    }

    /// Citation sources. A source that names a note makes a graph edge.
    /// A source that names no note is an external key, for example a
    /// DOI, a book, or a URL. An external key is never a broken link.
    fn optional_references(&self, doc: &Note, member: usize, graph: &StoreGraph) -> Vec<StoreRef> {
        let aliases = graph.config(member).aliases();
        crate::provenance::citation_refs(doc.frontmatter.provenance.as_deref(), &doc.body)
            .iter()
            .map(|source| StoreRef::parse(source, &aliases))
            .collect()
    }

    fn resolve_local(&self, id: &str, entries: &[Entry<Note>]) -> Option<usize> {
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        resolve_within(id, &ids).ok()
    }
}

/// Resolve an ID inside one store: exact, then 4-character prefix, then
/// slug. The vocabulary is zettel's, which is why it lives here.
fn resolve_within(input: &str, ids: &[&str]) -> Result<usize> {
    if let Some(i) = ids.iter().position(|id| *id == input) {
        return Ok(i);
    }

    if input.len() == 4
        && input
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        let prefix_dash = format!("{input}-");
        let matches: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter(|(_, id)| id.starts_with(&prefix_dash))
            .map(|(i, _)| i)
            .collect();
        match matches.len() {
            0 => {}
            1 => return Ok(matches[0]),
            _ => return Err(Error::AmbiguousPrefix(input.into())),
        }
    }

    let slug_matches: Vec<usize> = ids
        .iter()
        .enumerate()
        .filter(|(_, id)| mdstore::slug::extract_prefix(id).is_some_and(|(_, slug)| slug == input))
        .map(|(i, _)| i)
        .collect();
    if slug_matches.len() == 1 {
        return Ok(slug_matches[0]);
    }

    Err(Error::NoteNotFound(input.into()))
}

/// One closure member, for `store list`.
#[derive(Debug, serde::Serialize)]
pub struct StoreRow {
    /// Empty for the vantage store itself.
    pub alias: String,
    pub source: String,
    pub notes: usize,
    pub unavailable: Option<String>,
    /// True when a cache holds this store.
    pub remote: bool,
    /// The time since the last fetch, for a remote store.
    pub age: Option<String>,
}

/// A note as seen from a vantage: the note plus where it lives.
pub struct View<'a> {
    pub id: DocId,
    pub note: &'a Note,
    /// How the note prints and parses from this vantage: bare when
    /// local, `alias:id` or `alias/alias:id` when it belongs to a
    /// dependency.
    pub qualified: String,
    pub foreign: bool,
}

/// A loaded vantage.
pub struct Workspace {
    snapshot: Snapshot<NoteSource>,
    source: NoteSource,
    /// Dependencies that the machine-local registry redirected to a
    /// checkout, with that checkout's path.
    redirected: Vec<(String, String)>,
}

impl Workspace {
    /// Load the vantage store and its closure from what is on this
    /// machine. No command that reads reaches the network.
    pub fn open(root: &Utf8Path) -> Result<Self> {
        Self::open_with(root, false)
    }

    /// Load the closure, and clone a remote store that is absent.
    /// Only `store sync` uses this.
    pub fn open_fetching(root: &Utf8Path) -> Result<Self> {
        Self::open_with(root, true)
    }

    fn open_with(root: &Utf8Path, fetching: bool) -> Result<Self> {
        let registry = Registry::load().unwrap_or_default();
        let graph = if fetching {
            let locator = RegistryLocator::new(registry, FetchingLocator);
            let g = StoreGraph::open(root.as_std_path(), &locator)
                .map_err(crate::provenance::from_mdstore)?;
            (g, locator.redirected())
        } else {
            let locator = RegistryLocator::new(registry, LocalPaths);
            let g = StoreGraph::open(root.as_std_path(), &locator)
                .map_err(crate::provenance::from_mdstore)?;
            (g, locator.redirected())
        };
        let (graph, redirected) = graph;
        let snapshot =
            Snapshot::load(graph, &NoteSource).map_err(crate::provenance::from_mdstore)?;
        Ok(Workspace {
            snapshot,
            source: NoteSource,
            redirected: redirected.into_iter().map(|(u, p)| (u, p.display().to_string())).collect(),
        })
    }

    /// Notes of the vantage store only, in ID order.
    pub fn local(&self) -> Vec<View<'_>> {
        self.snapshot
            .member_documents(0)
            .map(|(id, e)| self.view(id, e))
            .collect()
    }

    /// Every note in the closure, vantage store first.
    pub fn all(&self) -> Vec<View<'_>> {
        self.snapshot
            .documents()
            .map(|(id, e)| self.view(id, e))
            .collect()
    }

    fn view<'a>(&'a self, id: DocId, entry: &'a Entry<Note>) -> View<'a> {
        View {
            id,
            note: &entry.doc,
            qualified: self.snapshot.qualify(id),
            foreign: self.snapshot.is_foreign(id),
        }
    }

    /// Find a note by a reference written at the vantage.
    ///
    /// Accepts what commands print: a bare local ID, `alias:id`, or the
    /// read-only `alias/alias:id` path form for a note reached through a
    /// dependency's own dependency.
    pub fn find(&self, input: &str) -> Result<View<'_>> {
        let aliases = self.snapshot.graph.config(0).aliases();
        let r = StoreRef::parse(input, &aliases);
        // A qualified reference names its store. Resolve there, and let
        // an ambiguous prefix surface as an ambiguity, not a miss.
        let mut member = 0usize;
        for alias in &r.alias {
            member = self
                .snapshot
                .graph
                .target_of(member, alias)
                .ok_or_else(|| Error::UndeclaredStore(alias.clone()))?;
        }
        let ids: Vec<&str> = self
            .snapshot
            .member_documents(member)
            .map(|(_, e)| e.id.as_str())
            .collect();
        let index = resolve_within(&r.id, &ids)?;
        let id = DocId { member, index };
        let entry = self
            .snapshot
            .get(id)
            .ok_or_else(|| Error::NoteNotFound(input.into()))?;
        Ok(self.view(id, entry))
    }

    /// The notes that reference this one, from this vantage. A
    /// dependency's own vantage would not see the vantage store's
    /// references, because it does not declare it.
    pub fn backlinks(&self, id: DocId) -> Vec<View<'_>> {
        self.snapshot
            .backlinks(id)
            .iter()
            .filter_map(|d| self.snapshot.get(*d).map(|e| self.view(*d, e)))
            .collect()
    }

    /// Notes with no references in either direction.
    pub fn orphans(&self) -> Vec<View<'_>> {
        self.snapshot
            .orphans()
            .into_iter()
            .filter_map(|d| self.snapshot.get(d).map(|e| self.view(d, e)))
            .collect()
    }

    /// A note and everything within `depth` hops, across store
    /// boundaries.
    pub fn neighborhood(&self, start: DocId, depth: usize) -> Vec<View<'_>> {
        self.snapshot
            .neighborhood(start, depth)
            .into_iter()
            .filter_map(|d| self.snapshot.get(d).map(|e| self.view(d, e)))
            .collect()
    }

    /// References that named no note, with the note that made them.
    pub fn dangling(&self) -> Vec<(View<'_>, String)> {
        self.snapshot
            .dangling
            .iter()
            .filter_map(|(from, r)| {
                self.snapshot
                    .get(*from)
                    .map(|e| (self.view(*from, e), r.to_string()))
            })
            .collect()
    }

    /// Stores in the closure that could not be loaded. Every
    /// graph-derived answer says so rather than answering as if the
    /// closure were whole.
    pub fn missing(&self) -> Vec<String> {
        self.snapshot.missing()
    }

    /// Notes and files skipped while loading, for `check`.
    pub fn skipped(&self) -> &[String] {
        &self.snapshot.skipped
    }

    /// Declarations this vantage makes that other clones could not
    /// follow. Only meaningful for a shared store.
    pub fn unshareable(&self, root: &Utf8Path) -> Vec<(String, String)> {
        self.snapshot
            .graph
            .config(0)
            .unshareable(root.as_std_path())
    }

    /// Citation sources that read as store references but name no note.
    ///
    /// A citation source is normally an opaque external key, so an
    /// unresolvable one is never a broken link. The exception is a
    /// source whose head matches a declared alias: it now reads as a
    /// cross-store reference, so either it is a typo or declaring that
    /// alias silently changed what an existing citation means. Either
    /// way it deserves surfacing once. A source that resolves is a
    /// working cross-store citation and says nothing.
    pub fn shadowed_citations(&self) -> Vec<(String, String)> {
        let aliases = self.snapshot.graph.config(0).aliases();
        if aliases.is_empty() {
            return Vec::new();
        }
        let mut found = Vec::new();
        for (_, entry) in self.snapshot.member_documents(0) {
            let sources = crate::provenance::citation_refs(
                entry.doc.frontmatter.provenance.as_deref(),
                &entry.doc.body,
            );
            for source in sources {
                let Some((head, _)) = source.split_once(':') else {
                    continue;
                };
                if !aliases.iter().any(|a| a == head) {
                    continue;
                }
                let r = StoreRef::parse(&source, &aliases);
                if self.snapshot.resolve_from(0, &r, &self.source).is_none() {
                    found.push((entry.id.clone(), source));
                }
            }
        }
        found
    }

    /// The aliases this vantage declares.
    pub fn aliases(&self) -> Vec<String> {
        self.snapshot.graph.config(0).aliases()
    }

    /// One row per closure member, for `store list`.
    pub fn store_members(&self) -> Vec<StoreRow> {
        self.snapshot
            .graph
            .members
            .iter()
            .enumerate()
            .map(|(i, m)| StoreRow {
                alias: m.alias_path.join("/"),
                remote: m.content.as_ref().is_some_and(|c| c.is_remote()),
                age: m.content.as_ref().and_then(|c| c.fetch_age()),
                source: if m.alias_path.is_empty() {
                    // The vantage store: show where it is, not the "."
                    // it was opened with.
                    m.content
                        .as_ref()
                        .and_then(|c| c.dir())
                        .map(|p| {
                            p.canonicalize()
                                .unwrap_or_else(|_| p.to_path_buf())
                                .display()
                                .to_string()
                        })
                        .unwrap_or_default()
                } else {
                    match &m.source {
                        mdstore::StoreSource::Path(p) => p.display().to_string(),
                        mdstore::StoreSource::Git { url, rev } => match rev {
                            Some(r) => format!("{url}@{r}"),
                            None => url.clone(),
                        },
                        mdstore::StoreSource::Blob { url } => url.clone(),
                    }
                },
                notes: self.snapshot.member_documents(i).count(),
                unavailable: m.unavailable.clone(),
            })
            .collect()
    }

    /// Refuse a write aimed at a dependency store.
    ///
    /// A dependency is another repository's working tree or a read-only
    /// cache. An edit through an alias changes a store that this store
    /// only reads.
    pub fn ensure_writable(&self, view: &View<'_>, input: &str) -> Result<()> {
        if view.foreign {
            return Err(Error::ForeignWrite(
                input.to_string(),
                self.store_label(view.id),
            ));
        }
        Ok(())
    }

    /// The store a member prints as.
    pub fn store_label(&self, id: DocId) -> String {
        let path = &self.snapshot.graph.members[id.member].alias_path;
        if path.is_empty() {
            String::new()
        } else {
            path.join("/")
        }
    }

    /// Fetch every declared remote store into the cache.
    pub fn sync_all(&self) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();
        for (i, member) in self.snapshot.graph.members.iter().enumerate() {
            if i == 0 || matches!(member.source, mdstore::StoreSource::Path(_)) {
                continue;
            }
            let alias = member.alias_path.join("/");
            let outcome = mdstore::store::sync_source(&member.source)
                .map_err(crate::provenance::from_mdstore);
            results.push((alias, outcome));
        }
        results
    }

    /// Dependencies that the registry redirected to a local checkout.
    ///
    /// A check that resolves through one of these proves nothing about
    /// any other machine: the checkout can hold commits that were never
    /// pushed.
    pub fn redirected(&self) -> &[(String, String)] {
        &self.redirected
    }

    /// References that resolve only because the registry redirected a
    /// dependency to a local checkout.
    ///
    /// This re-resolves the closure through the declared sources. A
    /// reference that resolves here but not there is a reference that
    /// works on this machine and nowhere else.
    pub fn override_only_refs(&self, root: &Utf8Path) -> Vec<(String, String)> {
        if self.redirected.is_empty() {
            return Vec::new();
        }
        // The declared closure: no registry, so a git dependency
        // resolves through the cache clone of its declared URL.
        let Ok(graph) = StoreGraph::open(root.as_std_path(), &LocalPaths) else {
            return Vec::new();
        };
        let Ok(declared) = Snapshot::load(graph, &NoteSource) else {
            return Vec::new();
        };
        let aliases = self.snapshot.graph.config(0).aliases();
        let mut found = Vec::new();
        for (id, entry) in self.snapshot.member_documents(0) {
            let _ = id;
            for text in NoteSource
                .references(&entry.doc, 0, &self.snapshot.graph)
                .iter()
                .map(|r| r.to_string())
            {
                let r = StoreRef::parse(&text, &aliases);
                if !r.is_foreign() {
                    continue;
                }
                let here = self.snapshot.resolve_from(0, &r, &self.source).is_some();
                let there = declared.resolve_from(0, &r, &NoteSource).is_some();
                if here && !there {
                    found.push((entry.id.clone(), text));
                }
            }
        }
        found
    }

    /// The knowledge base statistics, computed on the loaded graph.
    ///
    /// The orphan count and the connection counts come from the same
    /// graph that `orphans` and `backlinks` answer from. Computing them
    /// twice, in two places, produced two different definitions of an
    /// orphan and two answers that contradicted each other.
    pub fn stats(&self) -> crate::Stats {
        let local = self.local();
        let total = local.len();

        let mut span_counts = crate::SpanCounts::default();
        let mut tag_counts: Vec<(String, usize)> = Vec::new();
        for view in &local {
            for tag in &view.note.frontmatter.tags {
                if let Some(entry) = tag_counts.iter_mut().find(|(t, _)| t == tag) {
                    entry.1 += 1;
                } else {
                    tag_counts.push((tag.clone(), 1));
                }
            }
            let spans = crate::provenance::resolve_spans_lenient(
                view.note.frontmatter.provenance.as_deref(),
                &view.note.body,
            );
            for s in &spans {
                match s.marker.as_ref().map(|m| m.origin.as_str()) {
                    Some(crate::provenance::ORIGIN_HUMAN) => span_counts.human += 1,
                    Some(crate::provenance::ORIGIN_AGENT) => span_counts.agent += 1,
                    Some(crate::provenance::ORIGIN_CITATION) => span_counts.citation += 1,
                    _ => span_counts.unknown += 1,
                }
                if crate::provenance::is_unreviewed_agent(s.marker.as_ref()) {
                    span_counts.unreviewed_agent += 1;
                }
            }
        }
        tag_counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        let mut most_connected: Vec<(String, String, usize)> = local
            .iter()
            .map(|v| {
                (
                    v.qualified.clone(),
                    v.note.frontmatter.title.clone(),
                    self.backlinks(v.id).len(),
                )
            })
            .collect();
        most_connected.sort_by_key(|(_, _, count)| std::cmp::Reverse(*count));

        crate::Stats {
            total,
            span_counts,
            tag_counts,
            most_connected: most_connected.into_iter().take(5).collect(),
            orphan_count: self.orphans().len(),
        }
    }

    /// Files in this store that could not be parsed at all.
    ///
    /// The scan skips them, so they are absent from every command. The
    /// diagnostic command has to survive the corruption it diagnoses,
    /// which is why this walks the directory rather than the loaded
    /// notes.
    fn unparseable(&self, root: &Utf8Path) -> Vec<crate::BrokenLink> {
        let mut findings = Vec::new();
        let config_path = root.join("zettel.yml");
        let dir_name = std::fs::read_to_string(config_path.as_std_path())
            .ok()
            .and_then(|text| yaml_serde::from_str::<ZettelConfig>(&text).ok())
            .map_or_else(|| ".zettel".to_string(), |c| c.zettel_dir);
        let dir = root.join(&dir_name);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return findings;
        };
        let mut paths: Vec<std::path::PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .filter(|p| mdstore::store::is_regular_file(p))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Err(e) = crate::note::parse_note(&text) {
                findings.push(crate::BrokenLink {
                    source_id: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string(),
                    source_title: "unparseable".to_string(),
                    target: e.to_string(),
                    location: "frontmatter".to_string(),
                });
            }
        }
        findings
    }

    /// Every problem in the closure, as one list.
    ///
    /// This is the whole of `check`. Each interface presents the same
    /// findings: the CLI prints them, and a server returns them. A
    /// finding assembled in one interface and not the other is how the
    /// two drift apart.
    pub fn check(&self, root: &Utf8Path) -> Vec<crate::BrokenLink> {
        let mut findings = self.unparseable(root);

        // A note whose provenance the vocabulary does not accept. The
        // repo-wide commands degrade it to unknown; this names it.
        for view in self.local() {
            if let Err(e) = crate::provenance::resolve_spans(
                view.note.frontmatter.provenance.as_deref(),
                &view.note.body,
            ) {
                findings.push(crate::BrokenLink {
                    source_id: view.qualified.clone(),
                    source_title: view.note.frontmatter.title.clone(),
                    target: e.to_string(),
                    location: "provenance".to_string(),
                });
            }
        }

        // Every reference that named no note, local or cross-store. One
        // resolution path, so a link that works cannot be reported
        // broken.
        for (view, target) in self.dangling() {
            // Where the reference was written, so the reader knows what
            // to edit: a frontmatter links entry or a [[ref]] in the
            // body.
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
        for (alias, why) in self.unshareable(root) {
            findings.push(crate::BrokenLink {
                source_id: "stores.yml".to_string(),
                source_title: "store declarations".to_string(),
                target: format!("{alias}: {why}"),
                location: "declaration".to_string(),
            });
        }

        // A store that is declared but not there.
        for alias in self.missing() {
            findings.push(crate::BrokenLink {
                source_id: "stores.yml".to_string(),
                source_title: "store declarations".to_string(),
                target: alias,
                location: "unreachable store".to_string(),
            });
        }

        // A citation key that a newly declared alias now shadows: it
        // used to be an opaque external key and now reads as a store
        // reference.
        for (note_id, source) in self.shadowed_citations() {
            findings.push(crate::BrokenLink {
                source_id: note_id,
                source_title: "citation".to_string(),
                target: format!("'{source}' now reads as a store reference"),
                location: "shadowed citation".to_string(),
            });
        }

        // A reference that resolves only because the registry
        // redirected a dependency to a local checkout. It works here
        // and nowhere else.
        for (note_id, target) in self.override_only_refs(root) {
            findings.push(crate::BrokenLink {
                source_id: note_id,
                source_title: "registry override".to_string(),
                target: format!("'{target}' resolves only through a local checkout"),
                location: "override".to_string(),
            });
        }

        // A dependency the walk could not read at all.
        for finding in &self.snapshot.graph.findings {
            findings.push(crate::BrokenLink {
                source_id: "stores.yml".to_string(),
                source_title: "store declarations".to_string(),
                target: finding.clone(),
                location: "declaration".to_string(),
            });
        }

        // Files the guarded scan refused to read.
        for skipped in self.skipped() {
            findings.push(crate::BrokenLink {
                source_id: "store".to_string(),
                source_title: "skipped file".to_string(),
                target: skipped.clone(),
                location: "scan".to_string(),
            });
        }

        findings
    }

    /// The vantage store's root directory.
    pub fn root(&self) -> Utf8PathBuf {
        let root = self
            .snapshot
            .graph
            .root()
            .content
            .as_ref()
            .and_then(|c| c.dir())
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        Utf8PathBuf::from_path_buf(root).unwrap_or_default()
    }

    /// True when this vantage declares no dependencies: the single-store
    /// case, which must behave exactly as it did before composition.
    pub fn is_single_store(&self) -> bool {
        self.snapshot.graph.members.len() == 1
    }

}
