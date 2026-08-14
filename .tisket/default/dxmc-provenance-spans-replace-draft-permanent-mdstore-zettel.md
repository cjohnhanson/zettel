---
title: "provenance spans replace draft/permanent (mdstore + zettel)"
status: done
priority:
assignee:
labels: [feature]
depends_on: []
created: 2026-08-14T15:17:17Z
updated: "2026-08-14T19:51:46Z"
---

## End goal

Section-level provenance for markdown documents, shared by mdstore (parser) and zettel (consumer). Draft/permanent is removed from zettel; provenance replaces it.

## Model

A span's provenance is `origin[:qualifier] [key=value ...]`:

- `human[:name]` — the qualifier names the author.
- `agent[:kind]` — kind is `summary`, `index`, or `inference`.
- `citation[:source-key]` — verbatim quoted material; a note ID as key joins the link graph; external sources use a `src=` attribute.
- `reviewed=DATE` (optional `reviewer=NAME`) marks human approval of an agent span. Only tooling writes it.
- A missing provenance is **unknown** — never defaulted to human, so a forgotten key cannot mint human-authored text.

Markup: `<!-- prov SPEC -->` ... `<!-- /prov -->` in the body; a `provenance:` frontmatter key holds the default spec for unmarked text. A fully single-origin note carries zero markers.

## Layering decision

mdstore parses the generic shape (origin, qualifier, attrs) and stays vocabulary-agnostic. zettel enforces the vocabulary (valid origins, agent kinds). tisket adopts the same markers later (follow-up, out of scope here).

## Scope

1. mdstore: `provenance` module — spec parsing, marker parsing, `spans()` over a body, marker serialization for stamping.
2. zettel: remove `Status`; `--provenance` on create/edit; `note list --provenance/--unreviewed`; `read --provenance` span filtering; `note review <id> [--approve ...]`; `stats` provenance breakdown; `migrate` (status: permanent → provenance: human; draft → key removed); `check` reports invalid specs/markers.
3. missouri suite + docs updated.

## Success criteria

- Old notes with a `status:` key still parse (key survives in extra) and `migrate` converts them.
- cargo tests green in both repos; missouri suite green in preinstalled mode.
- A note mixing all three origins round-trips, reviews, and filters correctly end to end.

## Out of scope

Numeric trust weighting (labels only, no scores), tisket adoption, /zettel skill update in co.d (follow-up after merge + install).

## Scratch Notes

## QA plan (written before test code)

### mdstore: span parsing

1. Body with no markers → one span, no marker provenance, whole body.
2. One marked span in the middle → three spans: unmarked, marked, unmarked; text boundaries exact, marker lines excluded from span text.
3. Marker at the very start / very end of body.
4. Unclosed open marker → span runs to end of body; parses, no error.
5. A second open while a span is open → implicitly closes the first.
6. A stray `<!-- /prov -->` with no open span → left as plain body text.
7. Spec forms: `human`, `human:cody`, `agent:inference`, `citation:a3f2`, attrs `reviewed=2026-08-14 reviewer=cody`, `src=https://…` (value with colons), quoted value with spaces.
8. Malformed prov comment (`<!-- prov -->`, `<!-- prov foo=bar -->` no origin) → error, not silent text.
9. Empty body; body that is only a marker pair with empty interior.
10. Marker serialization round-trips: parse → to_string → parse gives the same marker.
11. `<!-- prov ... -->` inside a fenced code block IS parsed (documented v1 limitation) — test pins the behavior.

### zettel

1. `note create --provenance agent:summary` writes the key; omitted → no key; invalid (`agent:guess`, `robot`, `human extra junk`) → error, no file created.
2. `note list` line shows provenance (or `unknown`); `--provenance human` matches notes with ≥1 human span (default or marker); `--unreviewed` matches notes with ≥1 agent span lacking `reviewed=`.
3. `read --provenance human,citation` prints only matching spans; notes with no matching span omitted; `reviewed` token matches stamped agent spans.
4. `note review <id>` lists spans with indices + provenance + excerpt; `--approve` stamps `reviewed=DATE` on agent spans (frontmatter spec for default-covered text, marker rewrite for marked spans); approving a human span errors; `--approve all` stamps every unreviewed agent span; second approve is a no-op (already stamped, date unchanged? → re-stamp with today; simplest: overwrite).
5. `note edit --provenance human:cody` replaces the default spec; validates.
6. `migrate`: `status: permanent` → `provenance: human` (only when no provenance key present); `status: draft` → status key removed only; second run is a no-op; notes without status untouched.
7. A pre-migration note (status in extra) parses and all commands work.
8. `stats` shows span counts by origin + unreviewed agent count.
9. `check` reports an invalid provenance spec and a malformed marker with the note ID; repo-wide list/read do not die on one bad note (bad note counts as unknown).
10. e2e journey: agent creates note (agent:summary) → edit --append adds a citation span, an inference span, a human span → list --unreviewed shows it → review --approve the inference span → read --provenance human,citation,reviewed returns the right spans → --unreviewed still shows it (default summary spans unstamped) → approve all → gone from --unreviewed.

### Decisions taken while planning

- Malformed markers are hard errors from mdstore spans(); zettel repo-wide commands degrade that note to unknown and `check` names it.
- Review applies to agent spans only in v1.
- Approve indices cover all spans (stable display order); non-agent approval is an error, not a skip.
## State of play (2026-08-14)

Implementation complete, all staged, uncommitted, awaiting review.

- mdstore (~/Projects/mdstore, branch feat/provenance-spans, staged): `provenance` module — Marker (origin/qualifier/attrs), spec + marker-line parsing, parse_spans/render_spans with exact round-trip (whitespace separator spans kept), InvalidProvenance error. 50 tests green, clippy clean.
- zettel (branch feat/provenance, staged): Status removed everywhere; provenance module (vocabulary validation, resolved spans with raw indices, token matching, note-level fallback for empty bodies); CLI: create/edit -p, list -p/--unreviewed, read span filtering, note review [--approve all|N,N --reviewer], migrate, check reports invalid provenance, stats span counts, show JSON carries spans. 15 unit tests green, clippy clean.
- Missouri suite: 10/10 paths, 509 assertions, hermetic sandbox kept. States renamed/added: has-note-human (was has-note-permanent), has-provenance-spans, spans-approved, has-legacy-note, migrated.
- Issue txyn fixed as a side effect: stderr assertions now grep for the error line instead of exact-matching the stream, per the issue's stated fix direction. Close txyn when this branch merges.
- Docs rewritten: README, what-is-zettel, getting-started, cli-reference.

## Before merge

1. zettel Cargo.toml carries a TEMPORARY `[patch]` to ../mdstore. Remove it and pin the new mdstore rev after the mdstore PR merges.
2. Order: mdstore PR → merge → pin rev in zettel → zettel PR.
3. Follow-ups (not in this branch): tisket adopts the same markers; update the /zettel skill in co.d after merge + install.
## Adversarial test-gap sweep (2026-08-14, 13-agent workflow)

Full list: ~/.artifacts/zettel-missouri-tests/ (artifact) and scratchpad/missouri-gaps.json (structured). 64 raw findings → 36 canonical tests, 14 new missouri states, 10 distinct suspected bugs.

The bugs that matter (all probed against the real binary):
1. HIGH — one corrupt note (YAML-typed provenance, empty .md, duplicate key) bricks list/read/stats/orphans AND check, no filename in the error. Frontmatter deserialization bypasses the documented leniency. Same failure class the serde_yml fix was for.
2. HIGH — empty-body agent note: flagged by --unreviewed forever, --approve all stamps nothing (default only written when stamped > 0). Queue dead-end.
3. HIGH — --append into a body ending in an unclosed agent span: appended human text absorbed into the agent span and approvable as reviewed agent content. Mislabeling.
4. MED — edit --provenance silently drops reviewed=/reviewer= attrs even when the spec is unchanged.
5. MED — --approve <hidden separator index> stamps the whole default; explicit path lacks the is_separator check the all-branch has.
6. MED — approve path skips vocabulary validation (listing is strict, approve is not); invalid marker also hides a note from --unreviewed.
7. MED — malformed/unknown --where selectors silently dropped (typo lists everything).
8. LOW — doubled 'invalid provenance:' error prefix; duplicate approve indices double-count.

Design question, not a bug: citation:<note-id> does not join the link graph (no backlink, dangling source not a broken link). The design sketch said it would. Decide and pin either way.

~24 of the 36 tests are writable now; 12 are blocked on the bug/design decisions above (writing them today would pin the defects).
## Bug-fix + test batch (2026-08-14, after the adversarial sweep)

All 10 probed bugs fixed and verified against a rebuilt binary; citation-graph question settled by implementing the design sketch (a citation source that resolves to a note ID is a graph edge in backlinks/orphans/context; unresolvable sources are external keys, never broken links).

Fixes (zettel unless noted):
1. Error prefix no longer doubles (from_mdstore unwrap).
2. approve explicit-index path rejects hidden separator indices (SpanNotFound).
3. Duplicate approve indices stamp and count once; a shared default counts once.
4. Malformed/unknown --where selectors are errors (InvalidSelector), never silent.
5. approve validates marker vocabulary like the listing does.
6. One unparseable file no longer bricks repo-wide commands: list skips with a stderr warning; check names the file (location [frontmatter]).
7. Empty-body agent note: --approve all stamps the default; queue clears.
8. Review-stamp semantics: same-spec edit --provenance keeps stamps; different origin/qualifier drops them; any --body/--append edit drops the DEFAULT stamp (content changed); marker stamps stay.
9. --append closes an open trailing span first (mdstore ends_open); appended text cannot inherit agent provenance.

Missouri suite: 21/21 paths, 812 assertions (was 10/509). New states: cli-edges, has-marker-edge-notes, has-unclosed-span, has-bad-notes (corrupt + ambiguous-prefix fixtures), has-empty-agent-note, has-marked-no-default, has-two-agent-spans, has-citation-note, has-quoted-dangling-ref, has-unicode-provenance, has-invalid-marker. mdstore: +ends_open, 51 tests. Docs updated for citation edges, stamp invalidation, corrupt-file resilience.

Everything staged in both repos, uncommitted. Pre-merge steps unchanged (drop the Cargo.toml [patch], mdstore PR first, pin rev).
