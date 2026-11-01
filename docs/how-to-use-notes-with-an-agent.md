<!-- metadata
title: "How to use project notes with a coding agent"
description: "Let an agent find earlier research, record new findings, and reuse them in a fresh session."
type: how-to
-->

# How to use project notes with a coding agent

Give your coding agent access to the project's research through the
Zettel CLI. The agent needs to know when to look for notes and when to
save a finding. Keeping Markdown in the repository alone does not put
it into a conversation.

You need Zettel 0.3.0 and an agent that can run shell commands in your
project. The [first tutorial](getting-started.md) covers installation
and note creation. This guide uses your existing project.

## Open the project's store

In a terminal at the project root, check that the command is available:

```sh
zettel --version
```

If the project already contains `zettel.yml`, keep it and its configured
note directory. Otherwise, create the store:

```sh
zettel init
```

Inspect the project's notes explicitly from this directory:

```sh
zettel --root "$PWD" note list
zettel --root "$PWD" check
```

The list can be empty in a new store. The check should report
`no problems found`. If it reports missing targets, use
[the repair guide](how-to-repair-broken-links.md) before relying on them.

## Add note use to the agent's project instructions

Add the following section to the project instruction file your host
loads. Preserve its existing instructions. The ecosystem's
[project adoption guide](https://eng.rip/how-to-adopt-agent-work-in-a-project/)
explains host instruction loading and how to verify it; use that guide
if the project does not have an instruction file yet.

```markdown
## Project research

Use Zettel for findings that another task or session can reuse.
At the start of an investigation, run `zettel prime` for command guidance,
then search the project notes using terms from the task. Read relevant
notes in full with `zettel note show`; follow their sources and use
`zettel context` when linked findings matter. Check the evidence against
the current code before relying on an old conclusion.

Save useful new findings with `zettel note create`. Include the source
path or URL, revision when available, and the observed result. Use
`agent:summary` for a source summary and `agent:inference` for a new
conclusion. Link related notes. Leave human approval to the human.
```

`zettel prime` prints command guidance; it does not load notes or install
host instructions. An agent can use `zettel search 'retry|timeout'` to
find candidate notes, then read a returned ID with `zettel note show`.
Use terms appropriate to the actual task.

`search` and `read` operate on the current store. A declared dependency
does not join those searches. For research in another store, run the
search from its root or use `zettel --root /path/to/store search PATTERN`.
Note lookups and graph queries can follow qualified IDs such as
`handbook:a3f2`. [Composed stores](what-is-zettel.md#composed-stores)
explains those references.

## Verify discovery during ordinary work

Choose a task related to an existing note. Start a fresh agent session
in the project and give it the task in your usual words, without pasting
the note or its ID. Inspect the agent's actions: it should search the
store, read the relevant note, and compare its evidence with the current
project before making a decision.

If the store is empty, ask the agent to investigate a real project
question and record its finding. Read the result:

```sh
zettel note list --unreviewed
```

Use the returned ID with `zettel note show ID`. Check that the note
contains evidence and the appropriate provenance. Then open a fresh
session and ask a related question. The second session should find and
read the saved note before repeating the investigation.

If the agent skips the store, confirm that it loaded the project
instructions and can run `zettel` in its own shell. If a search misses
the note, inspect `zettel note list`, try terms from the title or tags,
and confirm the store root. A successful search with no matches does
not establish that no relevant research exists in another store.

## Preserve findings for later sessions

Review notes with [the agent-note review guide](how-to-review-agent-notes.md).
When evidence changes, [record a linked correction](how-to-correct-a-finding.md).
Include `zettel.yml`, the note directory, and the project instruction
changes in your normal Git review and commits. Another checkout needs
those files to recover the same research.

For an agent that uses MCP instead of shell access, the
[server reference](cli-reference.md#zettel-serve) describes the transport
and access modes. MCP note creation records agent provenance; it does
not expose human approval.
