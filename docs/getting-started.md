<!-- metadata
title: "Getting started with Zettel"
description: "Create a note store, link sample notes, and inspect their provenance before reuse."
type: tutorial
-->

# Getting started with Zettel

Create a small store of linked notes with explicit provenance. A store is a directory that holds `zettel.yml`. The notes in this exercise describe Zettel itself. You can inspect every claim before using it in another session.

You need git, a bash or zsh terminal, and Zettel 0.3.1. Install the pinned version with Cargo if you have Rust 1.88 or newer and a C compiler:

```sh
cargo install --locked zttl --version 0.3.1
```

Check the binary:

```console
$ zettel --version
zettel 0.3.1

```

If you do not use Rust, download the archive for your platform from the [0.3.1 release](https://github.com/cjohnhanson/zettel/releases/tag/v0.3.1). Extract it and put its `zettel` binary on your PATH. The [installation instructions](https://github.com/cjohnhanson/zettel/blob/v0.3.1/README.md#install) list other package managers.

## Initialize a practice repository

Make an empty directory and change into it, so the exercise does not change an existing store. Note its path. Then create a repository and a store:

```console
$ git init -q

$ zettel init
initialized a zettel store at [CWD]

```

Zettel prints the path of the new store. It creates `zettel.yml` and the `.zettel/` directory. Keep this terminal in the practice repository for the remaining commands.

## Write a note with a body

The command below labels the note as written by a human reader. If an agent runs the exercise, it must replace `-p human` with `-p agent:summary`. That note then joins the unreviewed list, and the later human-only read excludes it.

```console
$ zettel note create "Zettel keeps notes beside code" -t zettel,setup -p human -b 'zettel init creates zettel.yml and a .zettel directory in the working directory.'
ab12-zettel-keeps-notes-beside-code

```

The command prints the new note's ID: four generated characters, then the title's slug. Your four characters differ from the ones shown here. A command accepts the slug alone, so the rest of this tutorial uses `zettel-keeps-notes-beside-code`. The show command prints the note's metadata and body:

```console
$ zettel note show zettel-keeps-notes-beside-code
ab12-zettel-keeps-notes-beside-code

  Title:        Zettel keeps notes beside code
  Provenance:   human
  Tags:         zettel, setup

zettel init creates zettel.yml and a .zettel directory in the working directory.

```

Open the file under `.zettel/` in your editor to inspect the same Markdown file.

Provenance labels identify how text was produced; they do not establish that it is correct.

Find the note by tag and by body text:

```console
$ zettel note list --tag setup
ab12-zettel-keeps-notes-beside-code  human  [zettel, setup]  Zettel keeps notes beside code

$ zettel search 'zettel init'
ab12-zettel-keeps-notes-beside-code  Zettel keeps notes beside code  (body)

```

The list contains one note. The search result names that note and marks the match as `(body)`. If a later command reports a missing note, run `zettel note list` and use the actual ID.

## Connect a proposed use to the finding

Create a second sample note and mark it as an inference. This label demonstrates how to store a claim that still needs review. `-l` links it to the first note:

```console
$ zettel note create "Keep setup findings for the next session" -t zettel,workflow -p agent:inference -l zettel-keeps-notes-beside-code -b 'A later coding session could read these repository notes before repeating setup research.'
cd34-keep-setup-findings-for-the-next-session

$ zettel backlinks zettel-keeps-notes-beside-code
cd34-keep-setup-findings-for-the-next-session  Keep setup findings for the next session

```

The backlinks output names the second note because it links to the first. Read the connected notes together:

```console
$ zettel context keep-setup-findings-for-the-next-session --depth 1
--- cd34-keep-setup-findings-for-the-next-session ---
title: Keep setup findings for the next session
provenance: agent:inference
tags: zettel, workflow
links: zettel-keeps-notes-beside-code

A later coding session could read these repository notes before repeating setup research.

--- ab12-zettel-keeps-notes-beside-code ---
title: Zettel keeps notes beside code
provenance: human
tags: zettel, setup

zettel init creates zettel.yml and a .zettel directory in the working directory.

```

The output includes both titles and bodies. The link is stored in the second note's `links` frontmatter field.

## Inspect the inference before reusing it

```console
$ zettel note list --unreviewed
cd34-keep-setup-findings-for-the-next-session  agent:inference  [zettel, workflow]  Keep setup findings for the next session

$ zettel note review keep-setup-findings-for-the-next-session
cd34-keep-setup-findings-for-the-next-session  Keep setup findings for the next session

  1  agent:inference (default)  A later coding session could read these repository notes bef…

```

The list contains the inference note and any other notes with unreviewed agent content. The review output shows one numbered span with `agent:inference (default)` and a text preview. If the preview ends in `…`, read the full note with `zettel note show keep-setup-findings-for-the-next-session` before approving it.

Read only the human-authored content:

```console
$ zettel read --provenance human
--- ab12-zettel-keeps-notes-beside-code ---
title: Zettel keeps notes beside code
provenance: human
created: 2026-01-01T00:00:00Z
tags: zettel, setup

[human]
zettel init creates zettel.yml and a .zettel directory in the working directory.


```

For a human-created setup note, this returns its text and excludes the inference. If an agent created both notes, the read returns no content. A provenance filter selects content by its label; a human must still assess the claim.

If you are a human reader and accept the sample inference, approve it with your own reviewer name in place of `your-name`:

```console
$ zettel note review keep-setup-findings-for-the-next-session --approve all --reviewer your-name
1 span(s) approved

$ zettel read --provenance human,reviewed
--- ab12-zettel-keeps-notes-beside-code ---
title: Zettel keeps notes beside code
provenance: human
created: 2026-01-01T00:00:00Z
tags: zettel, setup

[human]
zettel init creates zettel.yml and a .zettel directory in the working directory.

--- cd34-keep-setup-findings-for-the-next-session ---
title: Keep setup findings for the next session
provenance: agent:inference reviewed=2026-01-01 reviewer=your-name
created: 2026-01-01T00:00:00Z
tags: zettel, workflow
links: zettel-keeps-notes-beside-code

[agent:inference reviewed=2026-01-01 reviewer=your-name]
A later coding session could read these repository notes before repeating setup research.


```

The read now includes the approved inference. It also includes the setup note if that note has human provenance. Only a human runs `--approve`. An agent following this tutorial must leave the inference unapproved and continue to the next section.

## Keep the files for another session

Append a checkpoint to the first note:

```console
$ zettel note edit zettel-keeps-notes-beside-code --append 'The practice store now has two linked notes.'

$ zettel note show zettel-keeps-notes-beside-code
ab12-zettel-keeps-notes-beside-code

  Title:        Zettel keeps notes beside code
  Provenance:   human
  Tags:         zettel, setup

zettel init creates zettel.yml and a .zettel directory in the working directory.

The practice store now has two linked notes.

$ zettel check
no problems found

```

The show output includes the appended sentence. `zettel check` checks store consistency, including links and provenance; it does not verify the truth of the prose.

Stage the configuration and notes:

```console
$ git add zettel.yml .zettel

$ git diff --cached --stat
 .zettel/ab12-zettel-keeps-notes-beside-code.md           | 14 ++++++++++++++
 .zettel/cd34-keep-setup-findings-for-the-next-session.md | 13 +++++++++++++
 zettel.yml                                               |  1 +
 3 files changed, 28 insertions(+)

```

The summary lists `zettel.yml` and two note files. If git already has your author identity configured, preserve them in a commit:

```console
$ git commit -q -m "Add linked setup notes"

```

The repository remains at the path you noted. In another terminal, change to that directory and run `zettel note list` to recover the note IDs. The records remain available without the original conversation.

Continue with a specific task:

- [Use notes with your coding agent](how-to-use-notes-with-an-agent.md) in
  an existing project, including reads in a fresh session.
- Check agent claims before reuse with [How to review agent notes](how-to-review-agent-notes.md). When evidence changes, follow its steps to correct a finding.
- [Repair broken links](how-to-repair-broken-links.md) when a target no longer resolves.

Use [the explanation](what-is-zettel.md) to understand provenance and the note graph. Use [the CLI reference](cli-reference.md) for editing, deletion, migration, and sharing stores.
