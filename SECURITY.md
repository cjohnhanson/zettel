# Security policy

## Reporting

Do not open a public issue for a vulnerability.

Report it privately:
**https://github.com/cjohnhanson/zettel/security/advisories/new**

That opens a thread only you and the maintainer can read.

Include what an attacker gains, what they must already control to get
it, the affected commit, and steps that reproduce it.

## What happens next

zettel has one maintainer, so response is best effort. Expect a reply
within a week.

A confirmed report gets a fix and an advisory published together. You
are credited unless you ask otherwise.

## Scope

zettel stores notes as markdown files with YAML frontmatter. Notes link
by ID, and a `stores.yml` declares the other stores a link can reach.
The tool also serves a store over the Model Context Protocol.

A declared store can come from content the reader does not control,
such as a vendored dependency or a remote store. A declaration decides
what the tool fetches and reads. The MCP server decides what a client
can ask for. Both are in scope.

In scope:

- A document, a declaration, or a name reaching outside the directory
  it should be confined to.
- A fetch reaching a host or a path that no declaration named.
- Reading untrusted content leading to code execution.
- The MCP server answering for a path outside the store it serves.
- A provenance marker claiming an origin its span did not have.

Out of scope:

- A dependency advisory with no exploitable path through this tool.
  Report it to that dependency.
- Denial of service from a malformed local file, where the caller
  already controls that file.

## Known boundaries

Documented limits are not vulnerabilities. The `confined` module of the
`mdstore-core` dependency holds the containment guarantee, and its
module documentation carries a `What this does not cover` section. Read
it before reporting a traversal issue.
