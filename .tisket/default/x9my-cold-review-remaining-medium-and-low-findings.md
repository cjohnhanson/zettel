---
title: 'cold review: remaining medium and low findings'
status: todo
priority: '3'
assignee: null
due_date: null
labels:
- review
depends_on: []
created: '2026-08-15T01:58:42Z'
updated: '2026-08-15T01:58:59Z'
---

## What this is

An independent cold review of the composition implementation, the library
refactor, and the three MCP servers. Seven reviewers, each with one lens,
then a second agent that tried to refute every finding. 52 findings
survived refutation; 39 remain after merging duplicates.

The critical finding and every high finding are fixed, merged, and
deployed. This issue holds what remains.

## Fixed already

1. tisket path traversal: an issue ID reached any markdown file on the host, read and write, through the server. (critical)
2. A remote store could declare a local path and pull a private directory into a reader closure.
3. zettel and tisket read and wrote through symlinked documents.
4. almanac let a dependency library key escape its store, on two code paths.
5. document_dir checked the text and not the resolved path.
6. StoreContent::read followed symlinks.
7. A served body could carry forged provenance markers.
8. note create -p and note edit -p accepted forged review stamps.
9. Blob and git shared a cache slot; a published index could overwrite a bare clone.
10. Two repositories could share one cache slot and serve each other content.
11. almanac signed the wrong bytes for a dependency skill file.

## Remaining

1. **[medium] One unreadable file or bad config anywhere in the closure aborts every command in the vantage store**
   DocumentSource::load's own contract says a bad document must be skipped and reported, but Snapshot::load propagates the per-member error with `?`, so a third-party dependency can deny service to the store that declares it — including to `check`, the command you would use to diagnose it.
   Fix: In mdstore/src/snapshot.rs:91-99, match on source.load(...) instead of using `?`: on Err, push format!("{}: {e}", label(member)) onto skipped and push an empty entry vector, mirroring the None arm. Separately, validate the configured document-dir string textually in NoteSource::load so the GitTree c

2. **[medium] A pinned revision that does not exist reads as an empty store instead of an error**
   resolve_rev uses bare `git rev-parse`, which succeeds for any 40-hex string whether the object exists or not, and list_tree turns every git failure into an empty file list, so a typo'd or force-pushed-away pin produces a store that reports present, healthy and empty.
   Fix: Use `git rev-parse --verify <rev>^{commit}` in resolve_rev. Every non-existent pin then takes the existing 'unavailable: …' path and the findings/check plumbing names it; list_tree's empty-on-error can stay for the genuine missing-prefix case.

3. **[medium] A registry override silently discards the declared rev pin while every surface still reports it**
   RegistryLocator returns StoreContent::Dir for a Git{url, rev} source and drops rev on the floor, so with an override in place the content served is the checkout's HEAD while `store list` still prints `<url>@v1.0` and `check` stays silent.
   Fix: Show the redirect in store_members(): print the checkout path with an '(overridden)' marker instead of the declared <url>@<rev>. In each check, emit one finding per redirected() entry whose declared source carried a rev: '<alias>: pinned to <rev> but resolved through local checkout <path>'.

4. **[medium] Identity dedup silently drops a declared store and resolves its alias to a different checkout**
   Two clones of one upstream at different revisions collapse into one closure member with no finding recorded; the second alias disappears from store list yet still resolves — to the first checkout's documents.
   Fix: In the dedup branch at store.rs:732-739, before `continue`, push a finding when the existing member's resolved location differs from this declaration's, naming both alias paths. Pair it with surfacing graph.findings so it reaches check and store list.

5. **[medium] tisket prints cross-store issue IDs that `tisket issue show` cannot resolve**
   The rollup prints a qualified child ID from the closure, but `issue show` goes through the single-tracker Repo instead of Workspace::find, so a printed ID does not parse back through the grammar it was printed in.
   Fix: Route IssueCommand::Show (cli.rs:632) and tisket_read_issue (serve.rs:146) through Workspace::find, which already handles unqualified IDs by resolving in member 0. Writes keep using Repo behind the existing ensure_writable foreign check.

6. **[medium] Approving one unmarked span silently approves every unmarked span, and the printed count under-reports it**
   approve_spans stamps an unmarked span by writing the stamp onto the note's shared frontmatter default, which covers every other unmarked span too, while reporting '1 span(s) approved'.
   Fix: In the None arm of approve_spans, when the caller named specific spans, set raw[i].marker to an explicit stamped clone of the default so render_spans wraps exactly the reviewed text and the other unmarked spans keep the unstamped default; increment stamped per span. Keep default-stamping only for --

7. **[medium] `note create -p` and `note edit -p` accept reviewed=/reviewer= attributes, so review can be forged without reviewing**
   parse_spec validates the origin and qualifier and never inspects attrs, and both create_note and edit_note store the spec as given — so any caller writes a review stamp in one command.
   Fix: Add validate_user_spec(spec) to provenance.rs: parse_spec, then reject any attr keyed ATTR_REVIEWED or ATTR_REVIEWER with 'a review stamp is written by zettel note review --approve'. Call it from create_note and edit_note in place of parse_spec; approve_spans keeps the unrestricted parse.

8. **[medium] An invalid inline marker launders agent content into 'unknown' and makes the note permanently un-reviewable**
   Nothing validates a body's markers on write, and every repo-wide command uses resolve_spans_lenient, which degrades a note that does not parse to a single unknown span — while `note review` uses the strict parser and errors out.
   Fix: Validate the body wherever it is written: in create_note call crate::provenance::resolve_spans(fm.provenance.as_deref(), body)? before serialize_note, and the same in edit_note once body/append have been applied. resolve_spans_lenient then covers pre-existing damage only.

9. **[medium] MCP zettel_context returns cross-store bodies with no provenance at all**
   The one served tool that crosses the trust boundary drops both the note's default provenance and its resolved spans, so a dependency's unreviewed agent inference reaches the model as bare prose in the same array shape as a human-authored vantage note.
   Fix: Build each element from v.note.view(&v.qualified) and insert the store label into that object, instead of hand-rolling a json! literal. That restores provenance and spans and removes the CLI/server divergence in one place.

10. **[medium] tisket_list_issues hardcodes closed:false, so a status filter for a terminal status returns an empty list**
   The served tool advertises a free-form `status` argument ('Only issues with this status') while passing closed: false with no way for a caller to change it, so filtering on a done status answers '[]' — reading as 'there are none' — for issues the same server returns on request.
   Fix: Parse status through Status::from_str, error naming the accepted values on failure, and derive the flag: let closed = status.map_or(false, Status::is_terminal) (Status::is_terminal exists at issue.rs:31-34). Add an optional boolean `closed` argument and apply the same derivation to the list_resource

11. **[medium] zettel_context declares `depth` as a string but reads only a JSON number**
   schema_with hardcodes "type": "string" for every property while the handler uses Value::as_u64, so a client that follows the published schema gets the default depth silently and only a schema-violating client gets the depth it asked for.
   Fix: Give schema_with a per-field type and declare depth as {"type":"integer","minimum":1,"maximum":10}. Keep a lenient reader (as_u64().or_else(|| v.as_str()?.parse().ok())) and return an explicit error when depth is present but unparseable, instead of falling back to 2.

12. **[medium] tisket silently discards a malformed --where selector (and --tag) and returns the unfiltered list with exit 0**
   Selector::parse returns an Option and the CLI collects it with filter_map, so an unparseable selector is dropped without a word and the full list reads as 'these are all the matches'.
   Fix: Add parse_selector(&str) -> Result<Selector> to tisket/src/selector.rs returning Error::InvalidSelector when Selector::parse yields None or the namespace is empty, and collect with .map(parse_selector).collect::<Result<Vec<_>>>()?. Apply the same to --tag: error on an entry with no `=` naming the KE

13. **[medium] zettel's orphans and stats use two different orphan definitions and contradict each other**
   Two implementations of the same question survive: Workspace::orphans uses resolved graph edges, Repo::orphans counts any non-empty links: frontmatter as connected even when it resolves to nothing. The CLI's orphans command uses the first and stats uses the second.
   Fix: Take orphan_count and most_connected in stats from the workspace graph — pass a &Workspace into Repo::stats or compute those two fields in the CLI's stats path, which already opens a Workspace — then delete Repo::orphans, Repo::backlinks and the now-callerless Repo::context.

14. **[medium] zettel's partial-closure caveat lives only in cli.rs, so the MCP server presents a truncated graph as whole**
   The rule that an incomplete-closure graph answer must say so is implemented as a println helper in the CLI; the served zettel_context and zettel_backlinks return the truncated graph with no marker, and zettel serves no check tool (tisket and almanac both do), so an MCP client cannot learn the closure is broken.
   Fix: Make the caveat data: have the neighborhood/backlinks accessors return the view list together with ws.missing(). The CLI keeps printing it to stderr; serve.rs emits it as an `unreachable` field on both results. Add a zettel_check tool returning Workspace::check for parity.

15. **[medium] almanac's red-flag scanner follows symlinked directories, so a hostile skill makes `almanac add` walk the whole filesystem**
   flags::walk recurses on p.is_dir(), which resolves symlinks, so the one step whose job is to catch hostile content before vendoring can be sent into an unbounded traversal and never returns.
   Fix: In flags.rs:38-50 take entry.file_type() once, skip is_symlink() entries with an explicit Flag ('symlink, not scanned') so the reviewer sees them, and recurse only when ft.is_dir(). Add a depth cap as a backstop.

16. **[medium] `almanac add` copies an escaping symlink into the project before any hash check and leaves it after refusing the skill**
   vendor.rs justifies copy_tree not re-checking symlink targets 'because hash_tree runs first'; in the add path that ordering is reversed, so an escaping symlink from an unreviewed third-party repo is materialized inside the user's tree and persists after the user declines.
   Fix: Call vendor::hash_tree(&skill_src) immediately after resolving skill_src and before flags::scan at ops.rs:148, propagating its error so an escaping symlink aborts add before anything is written. Then delete .almanac-staged/<name> on the 'not accepted' return, or stage under a tempdir handle that dro

17. **[low] A remote store's relative path dependency resolves against the process working directory**
   StoreGraph::open sets the declaring root to an empty PathBuf for any member with no local directory (GitTree::dir() is None), and LocalPaths::locate then joins the declared relative path onto "", which the OS resolves against the CWD — so the same store graph is not the same graph twice.
   Fix: Subsumed by rejecting Path sources declared by remote members. As a belt-and-braces guard, make LocalPaths::locate return Err('a relative path source has no declaring directory') when declaring_root is empty and the path is relative, instead of joining onto "".

18. **[low] StoreGraph::findings is never read, so an unreadable dependency stores.yml silently truncates the closure**
   mdstore substitutes an empty config when a dependency's stores.yml cannot be parsed and records the reason in graph.findings, but no consumer reads that field, so check reports a clean bill of health while part of the declared graph is missing.
   Fix: Add pub fn graph_findings(&self) -> &[String] to each Workspace returning self.snapshot.graph.findings, and in each check emit one finding per entry with location 'declaration' and source_id 'stores.yml', exactly as missing() is surfaced today.

19. **[low] `shared: true` escaping path declarations are only warned about, never rejected at resolution**
   The code documents that a shared store's outside path dependency 'is rejected at resolution rather than warned about', but StoreGraph::open never reads StoresConfig::shared; only check consults unshareable.
   Fix: Cheapest correct move is to make the comment true: rewrite store.rs:393-400 to say the check is reported by check, not enforced at resolution. If enforcement is wanted, have StoreGraph::open consult configs[cursor].shared and mark a rejected declaration unavailable with a finding instead of locating

20. **[low] The shared git cache has no locking, so concurrent syncs fail and a partial slot wedges permanently**
   ensure_clone tests for a HEAD file and then clones straight into the shared slot with no lock or temp-then-rename, so concurrent syncs race into one directory and a directory that merely contains HEAD is treated as a finished clone forever after.
   Fix: Clone into <slot>.tmp-<pid> and rename into place only after git exits 0, which makes a partial clone invisible and concurrent clones idempotent. Add a completeness test to is_cached: treat a slot that fails `git -C <slot> rev-parse --git-dir` as absent so a wedged cache self-repairs.

21. **[low] An https blob sync never prunes, so a retracted document lives in every consumer's cache forever**
   The https blob path writes each name the index lists and deletes nothing, so a document removed upstream — including one retracted as wrong or confidential — keeps being served and counted, with no way for the publisher to withdraw it.
   Fix: For https, sync into <slot>.new and rename it over the slot once every index entry is fetched — pruning and crash-atomicity in one change. Add --delete to the aws s3 sync and --delete-unmatched-destination-objects to the gcloud rsync.

22. **[low] zettel's tool surface is vantage-scoped while its resource and read surfaces span the closure, and the descriptions promise the closure**
   zettel_list_notes and zettel_search go through Repo (the vantage only) while list_resources uses ws.all() and zettel_read_note/zettel_context use the closure — yet the tool descriptions say 'the notes this knowledge base reaches' and the server instructions tell the client that resources and tools return the same content.
   Fix: Make the words match the code: 'List the notes this store owns' and 'Search the notes this store owns', and add a store field to each row so the scope is legible in the payload. Keeping the Repo backing preserves the claim-4 isolation of dependency provenance counts.

23. **[low] zettel's `check` is assembled in cli.rs, contradicting the library's claim to be the whole of check**
   Workspace::check's doc comment says it is the whole of check and that each interface presents the same findings, but cli.rs unions it with Repo::check(), the only source of provenance-vocabulary and unparseable-frontmatter findings — so a server trusting the documented entry point loses them.
   Fix: Move the two Repo::check producers into Workspace::check, which already holds member 0's documents — emit the provenance-vocabulary finding from the loaded notes and drop the separate unparseable-file walk in favour of the existing skipped() findings — then reduce the CLI's Check arm to calling ws.c

24. **[low] zettel labels every store-layer failure 'invalid provenance'**
   provenance::from_mdstore's catch-all arm wraps any mdstore error in Error::InvalidProvenance, so I/O failures, guard rejections and config errors all print under a label with nothing to do with them, sending anyone debugging a store problem into the provenance code.
   Fix: Add #[error("store: {0}")] Store(String) to zettel/src/error.rs and map the catch-all arm onto it, leaving InvalidProvenance for mdstore::Error::InvalidProvenance alone. Once per-member failures route into Snapshot::skipped with the member label, the alias appears in the message for free.

25. **[low] zettel's MCP argument errors report a missing or mistyped argument as a note that was not found**
   Missing-argument checks are built from Error::NoteNotFound, whose Display is "note '{0}' not found", so the diagnostic string becomes the note ID — and because the reader is as_str, a wrong-typed argument is reported as missing.
   Fix: Add Error::MissingArgument{tool, arg} and Error::UnknownTool(String) to zettel/src/error.rs and use them at those six sites. Check args.get(key) for presence before as_str, and report "'id' must be a string" when present but wrong-typed.

26. **[low] `zettel check` calls every finding a broken link, including unreachable stores and skipped files**
   Workspace::check returns dangling references, unshareable declarations, unreachable stores, shadowed citations, override-only refs and unreadable files all as BrokenLink values, and the CLI prints 'N broken link(s):' over the lot — inflating the count and misdescribing the problem.
   Fix: Rename BrokenLink to Finding with a kind field mirroring almanac's, print 'N problem(s):' grouped by kind (and 'no problems found'), and suppress the scan entry for an alias already reported under 'unreachable store'.

27. **[low] almanac drops an unreadable skill file from its published digest manifest without reporting it**
   collect swallows both directory and file read failures, so a file the process cannot read is omitted from the skill's resource list and the SEP-2640 listing presents an incomplete manifest as authoritative.
   Fix: Propagate the io error out of collect (it already returns Result) for the file-read case, or thread a skipped: &mut Vec<String> through skill_files/collect and surface those entries as a scan kind in Workspace::check, matching how SkillSource::load already reports an unreadable SKILL.md.

28. **[low] `tisket hooks setup` panics through todo!() on a documented subcommand**
   A subcommand that is declared, shown in --help and shell-completable has todo!() as its body, so it aborts with a Rust panic at exit 101 instead of returning a clean error.
   Fix: Replace the todo!() with Err(Error::General("hooks setup is not implemented yet".into())) so it exits 1 through main's error path, and mark the subcommand #[command(hide = true)] until it exists.

## Claims the code did not keep

- Claim 1: Direction does not hold and the closure is not bounded by declarations. A store fetched over git can declare `path: /anything` and that directory becomes a closure member whose documents are printed (mdstore/src/store.rs:698-731, LocalPaths::locate at :598-611) — reproduced in both zettel and tisket. A `shared: true` store's outside path dependency is served by every read command and only mentione
- Claim 2: Identity is the canonicalized source string, and that canonicalization is lossy: canonical_url (store.rs:271-283) strips the scheme and a trailing `.git` and lowercases, so two genuinely different repositories become one identity and one cache slot, and a consumer reads the wrong repository's documents while its sync reports success. Dedup does terminate cycles, but when it fires on two checkouts 
- Claim 3: Kept. Alias tables are per declaring member: StoreGraph::targets is keyed by (declaring member, alias) (mdstore/src/store.rs:669, :804), Snapshot::resolve_from walks from the referring document's member (snapshot.rs:168-174), and each tool parses a document's references against graph.config(member).aliases() (zettel/src/workspace.rs:73-99). Bare refs resolve in their own member via resolve_local. 
- Claim 4: The isolation half holds — dependency counts and unqualified filters stay vantage-scoped — but the review half does not. `note create -p` and `note edit -p` write reviewed=/reviewer= attributes verbatim (provenance.rs:43-57, repo.rs:224-227, :327-344), so review is forgeable in one command without ever running review. The MCP server writes the caller's body unparsed, so a remote agent stamps human
- Claim 5: Broken on both halves. Escape: almanac applies no containment guard at all to a dependency's `library:` value (workspace.rs:107-116), and mdstore's own document_dir guard validates the configured string but not the resolved path, so a symlinked `.zettel` directory escapes the root it was checked against (store.rs:849-862). Non-regular files: only mdstore's scan_documents skips by dirent type; zett
- Claim 6: Bare clones are keyed by a lossy canonicalization, so two different URLs share one slot and the first to populate it wins — a consumer reads a repository it never declared, and its fetch runs against the other URL's origin remote. Blob stores share that same slot namespace and the https blob writer accepts any index-supplied filename, so a blob sync overwrites a bare clone's `config` and `packed-r
- Claim 7: A registry override does change what the store effectively declares: RegistryLocator returns a Dir for a Git{url, rev} source and discards the rev (registry.rs:113-134), so a pinned consumer silently reads the checkout's HEAD while `store list` still prints <url>@<rev> and `check` reports nothing. The narrower half of the claim does hold — check does warn on a ref that resolves only through a loca
- Claim 8: Read-only-by-default holds, and approval is not exposed as a tool. Everything else fails. tisket's server allows far more than scratch-append: the id argument traverses out of the store, so an unauthenticated caller reads and writes arbitrary .md files on the host. zettel's server does not in fact refuse non-agent provenance — it stamps the frontmatter default and then writes the caller's body ver
- Claim 9: The CLI is not thin and the two interfaces disagree. almanac's CLI reimplements the closure in cli.rs and cannot see a git-declared library that the server serves (`almanac show gitskill` → not found while almanac_get_skill returns its full text), and its symlink-following scan gives `almanac list` a skill set the server returns as []. tisket's `issue show` and served read use Repo instead of Work
