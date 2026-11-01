<!-- metadata
title: "Getting started with Zettel"
description: "Create a repository knowledge base, link sample notes, and inspect their provenance before reuse."
type: tutorial
-->

# Getting started with Zettel

Create a small knowledge base with linked notes and explicit provenance. The notes in this exercise describe Zettel itself. You can inspect every claim before using it in another session.

You need git, a bash or zsh terminal, and Zettel 0.3.0. Install the pinned version with Cargo if you have Rust 1.88 or newer and a C compiler:

```sh
cargo install --locked zttl --version 0.3.0
zettel --version
```

The version command prints `zettel 0.3.0`. If you do not use Rust, download the archive for your platform from the [0.3.0 release](https://github.com/cjohnhanson/zettel/releases/tag/v0.3.0). Extract it and put its `zettel` binary on your PATH. The [installation instructions](https://github.com/cjohnhanson/zettel/blob/v0.3.0/README.md#install) list other package managers.

## Initialize a practice repository

Create a temporary directory so the exercise does not change an existing store:

```sh
zettel_demo=$(mktemp -d)
cd "$zettel_demo"
git init
zettel init
```

Zettel prints `initialized a zettel store at <path>`. It creates `zettel.yml` and the `.zettel/` directory. Keep this terminal in the practice repository for the remaining commands.

## Write a note with a body

Capture the generated ID in a shell variable:

```sh
setup_note=$(zettel note create "Zettel keeps notes beside code" \
  -t zettel,setup -p human \
  -b 'zettel init creates zettel.yml and a .zettel directory in the working directory.')
printf '%s\n' "$setup_note"
zettel note show "$setup_note"
```

The ID starts with four generated characters, followed by the title's slug. The show command prints the note's metadata and body. Open `.zettel/$setup_note.md` in your editor to inspect the same Markdown file.

The `-p human` flag describes the source of this exercise note. An agent that writes its own summary must use `agent:summary`. Provenance labels identify how text was produced; they do not establish that it is correct.

Find the note by tag and by body text:

```sh
zettel note list --tag setup
zettel search 'zettel init'
```

The list contains one note. The search result names that note and marks the match as `(body)`. If a later command reports an empty or missing ID, run `zettel note list` and use the actual ID; shell variables do not survive closing the terminal.

## Connect a proposed use to the finding

Create a second sample note and mark it as an inference. This label demonstrates how to store a claim that still needs review:

```sh
use_note=$(zettel note create "Keep setup findings for the next session" \
  -t zettel,workflow -p agent:inference -l "$setup_note" \
  -b 'A later coding session could read these repository notes before repeating setup research.')
printf '%s\n' "$use_note"
zettel backlinks "$setup_note"
```

The backlinks output names the second note because it links to the first. Read the connected notes together:

```sh
zettel context "$use_note" --depth 1
```

The output includes both titles and bodies. The link is stored in the second note's `links` frontmatter field.

## Inspect the inference before reusing it

```sh
zettel note list --unreviewed
zettel note review "$use_note"
```

The list contains the inference note. The review output shows one numbered span with `agent:inference (default)` and a text preview. If the preview ends in `…`, read the full note with `zettel note show "$use_note"` before approving it.

Read only the human-authored content:

```sh
zettel read --provenance human
```

This returns the setup note's text and excludes the inference. A provenance filter selects content by its label; a human must still assess the claim.

If you are a human reader and accept the sample inference, approve it with your own reviewer name in place of `your-name`:

```sh
zettel note review "$use_note" --approve all --reviewer your-name
zettel read --provenance human,reviewed
```

The approval prints `1 span(s) approved`. The read now includes the inference along with the setup note. Only a human runs `--approve`. An agent following this tutorial must leave the inference unapproved and continue to the next section.

## Keep the files for another session

Append a checkpoint to the first note:

```sh
zettel note edit "$setup_note" --append 'The practice store now has two linked notes.'
zettel note show "$setup_note"
zettel check
```

The show output includes the appended sentence. The check prints `no problems found`. It checks store consistency, including links and provenance; it does not verify the truth of the prose.

Stage the configuration and notes:

```sh
git add zettel.yml .zettel
git diff --cached --stat
```

The summary lists `zettel.yml` and two note files. If git already has your author identity configured, preserve them in a commit:

```sh
git commit -m "Add linked setup notes"
```

The repository remains at `$zettel_demo`. In another terminal, change to that directory and run `zettel note list` to recover the note IDs. The records remain available without the original conversation or shell variables.

Continue with a specific task:

- [Review agent notes](how-to-review-agent-notes.md) before reusing their claims.
- [Correct a finding](how-to-correct-a-finding.md) when the evidence changes.
- [Repair broken links](how-to-repair-broken-links.md) when a target no longer resolves.

Use [the explanation](what-is-zettel.md) to understand provenance and the note graph. Use [the CLI reference](cli-reference.md) for editing, deletion, migration, and sharing stores.
