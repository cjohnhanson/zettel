//! Serving a knowledge base over MCP.
//!
//! The same library the command line reads is what a client reads. This
//! module maps requests onto [`crate::workspace::Workspace`] and
//! [`crate::Repo`], and shapes the answers.
//!
//! Serving changes one thing that the command line cannot enforce.
//! Provenance is a convention there: an agent is asked to pass
//! `-p agent:summary`, and nothing stops it forgetting, or writing
//! `human:cody`. A server knows its caller is not the person who owns
//! the store, so it stamps `agent` provenance itself and refuses any
//! other. Approval is not offered at all: approving content is a human
//! act, and the human has the command line.

use std::path::Path;
use std::sync::Arc;

use camino::Utf8PathBuf;
use mdstore::mcp::{Access, DocUri, ServeConfig, Surface};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData as McpError,
    Implementation, InitializeResult, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, Resource, ResourceContents, ResourcesCapability, ServerCapabilities, Tool,
    ToolsCapability,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::workspace::Workspace;

const SCHEME: &str = "zettel";
const KIND: &str = "note";

/// A served knowledge base.
#[derive(Clone)]
pub struct ZettelServer {
    config: Arc<ServeConfig>,
}

impl ZettelServer {
    #[must_use]
    pub fn new(config: ServeConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    fn root(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.config.root.clone()).unwrap_or_default()
    }

    fn workspace(&self) -> Result<Workspace> {
        Workspace::open(&self.root())
    }

    /// The URI a note answers to, qualified by store when it belongs to
    /// a dependency.
    fn uri_of(qualified_id: &str, aliases: &[String]) -> String {
        DocUri::new(
            SCHEME,
            KIND,
            mdstore::store::StoreRef::parse(qualified_id, &aliases.to_vec()),
        )
        .to_string()
    }

    /// The note a URI names.
    fn note_id_of(&self, uri: &str) -> Result<String> {
        let ws = self.workspace()?;
        let aliases = ws.aliases();
        let parsed = DocUri::parse(uri, &aliases).map_err(crate::provenance::from_mdstore)?;
        if parsed.scheme != SCHEME || parsed.kind != KIND {
            return Err(Error::NoteNotFound(uri.to_string()));
        }
        Ok(parsed.reference.to_string())
    }

    // -- tools --

    fn tool_definitions(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool::new(
                "zettel_list_notes",
                "List the notes this knowledge base reaches, with their provenance. Filter \
                 by tag, or by provenance tokens such as human, agent, citation, reviewed, \
                 unknown.",
                Arc::new(schema_with(&[
                    ("tag", "Only notes with this tag.", false),
                    ("provenance", "Comma-separated provenance tokens.", false),
                ])),
            ),
            Tool::new(
                "zettel_read_note",
                "Return one note by ID, with its spans and the provenance of each, so each \
                 piece of text can be weighed by who wrote it.",
                Arc::new(schema_with(&[("id", "The note ID.", true)])),
            ),
            Tool::new(
                "zettel_search",
                "Search the notes with a regular expression, across titles, tags, and \
                 bodies.",
                Arc::new(schema_with(&[("pattern", "A regular expression.", true)])),
            ),
            Tool::new(
                "zettel_context",
                "Return a note and the notes within N hops of it, across store boundaries.",
                Arc::new(schema_with(&[
                    ("id", "The note ID to start from.", true),
                    ("depth", "How many hops. The default is 2.", false),
                ])),
            ),
            Tool::new(
                "zettel_backlinks",
                "Return the notes that reference this one.",
                Arc::new(schema_with(&[("id", "The note ID.", true)])),
            ),
        ];
        if self.config.access.writable() {
            tools.push(Tool::new(
                "zettel_create_note",
                "Write a new note. The server records it as agent-written: provenance is \
                 stamped by the server, not chosen by the caller, and a human approves it \
                 later at the command line.",
                Arc::new(schema_with(&[
                    ("title", "The note title.", true),
                    ("body", "The note body.", false),
                    ("tags", "Comma-separated tags.", false),
                    (
                        "kind",
                        "The kind of agent content: summary, index, or inference. The \
                         default is summary.",
                        false,
                    ),
                ])),
            ));
        }
        tools
    }

    fn call(&self, name: &str, args: &Map<String, Value>) -> Result<String> {
        let text = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_string);
        match name {
            "zettel_list_notes" => {
                let repo = crate::Repo::open(&self.root())?;
                let notes = repo.list_notes(&crate::ListNotesFilter {
                    tag: text("tag").as_deref(),
                    provenance: text("provenance").as_deref(),
                    unreviewed: false,
                })?;
                let listed: Vec<Value> = notes
                    .iter()
                    .map(|n| {
                        json!({
                            "id": n.id,
                            "title": n.frontmatter.title,
                            "provenance": n.frontmatter.provenance,
                            "tags": n.frontmatter.tags,
                        })
                    })
                    .collect();
                Ok(pretty(&listed))
            }
            "zettel_read_note" => {
                let id = text("id").ok_or_else(|| Error::NoteNotFound("id is required".into()))?;
                let ws = self.workspace()?;
                let view = ws.find(&id)?;
                Ok(pretty(&view.note.view(&view.qualified)))
            }
            "zettel_search" => {
                let pattern = text("pattern")
                    .ok_or_else(|| Error::NoteNotFound("pattern is required".into()))?;
                let repo = crate::Repo::open(&self.root())?;
                let results: Vec<Value> = repo
                    .search(&pattern)?
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.note.id,
                            "title": r.note.frontmatter.title,
                            "matched": r.matched_fields,
                        })
                    })
                    .collect();
                Ok(pretty(&results))
            }
            "zettel_context" => {
                let id = text("id").ok_or_else(|| Error::NoteNotFound("id is required".into()))?;
                let depth = args
                    .get("depth")
                    .and_then(Value::as_u64)
                    .unwrap_or(2)
                    .min(10) as usize;
                let ws = self.workspace()?;
                let start = ws.find(&id)?;
                let views: Vec<Value> = ws
                    .neighborhood(start.id, depth)
                    .iter()
                    .map(|v| {
                        json!({
                            "id": v.qualified,
                            "title": v.note.frontmatter.title,
                            "store": ws.store_label(v.id),
                            "body": v.note.body,
                        })
                    })
                    .collect();
                Ok(pretty(&views))
            }
            "zettel_backlinks" => {
                let id = text("id").ok_or_else(|| Error::NoteNotFound("id is required".into()))?;
                let ws = self.workspace()?;
                let target = ws.find(&id)?;
                let views: Vec<Value> = ws
                    .backlinks(target.id)
                    .iter()
                    .map(|v| json!({ "id": v.qualified, "title": v.note.frontmatter.title }))
                    .collect();
                Ok(pretty(&views))
            }
            "zettel_create_note" => {
                self.config
                    .ensure_writable()
                    .map_err(crate::provenance::from_mdstore)?;
                let title = text("title")
                    .ok_or_else(|| Error::NoteNotFound("title is required".into()))?;
                // The server stamps the provenance. A caller cannot ask
                // for `human`, and cannot omit it: this note was
                // written by an agent, and the store should say so.
                let kind = text("kind").unwrap_or_else(|| "summary".to_string());
                if !crate::provenance::AGENT_KINDS.contains(&kind.as_str()) {
                    return Err(Error::InvalidProvenance(format!(
                        "'{kind}' is not an agent kind; use summary, index, or inference"
                    )));
                }
                // The body is written by the caller. A marker in it
                // would claim a provenance the server did not stamp:
                // an inline `<!-- prov human:cody -->` span, or a
                // forged `reviewed=` attribute. The server refuses a
                // body that carries any marker, so the note's
                // provenance is exactly what the server recorded.
                if let Some(body) = text("body")
                    && mdstore::provenance::parse_spans(&body)
                        .map(|spans| spans.iter().any(|s| s.marker.is_some()))
                        .unwrap_or(true)
                {
                    return Err(Error::InvalidProvenance(
                        "the server stamps provenance; a body may not carry \
                         <!-- prov --> markers"
                            .into(),
                    ));
                }
                let repo = crate::Repo::open(&self.root())?;
                let id = repo.create_note(
                    &title,
                    crate::CreateNoteOptions {
                        tags: text("tags"),
                        links: None,
                        body: text("body"),
                        provenance: Some(format!("agent:{kind}")),
                    },
                )?;
                Ok(format!(
                    "created {id} with provenance agent:{kind}; a human approves it with \
                     'zettel note review {id} --approve all'"
                ))
            }
            other => Err(Error::UnknownField(other.to_string())),
        }
    }
}

fn pretty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn schema_with(fields: &[(&str, &str, bool)]) -> Map<String, Value> {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (name, description, is_required) in fields {
        properties.insert(
            (*name).to_string(),
            json!({ "type": "string", "description": description }),
        );
        if *is_required {
            required.push(json!(name));
        }
    }
    let mut schema = Map::new();
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".into(), Value::Array(required));
    }
    schema
}

fn to_mcp_error(e: &Error) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

impl ServerHandler for ZettelServer {
    fn get_info(&self) -> InitializeResult {
        let mut capabilities = ServerCapabilities::default();
        if self.config.surfaces.has(Surface::Tools) {
            capabilities.tools = Some(ToolsCapability::default());
        }
        if self.config.surfaces.has(Surface::Resources) {
            capabilities.resources = Some(ResourcesCapability::default());
        }

        let mut instructions = format!(
            "A zettelkasten knowledge base. Surfaces: {}. Every note carries a provenance: \
             who wrote each piece of text, and what kind of claim it makes. Weigh human \
             text as the author's own, a citation as quoted, and an unreviewed agent \
             inference as a hypothesis.",
            self.config.surfaces
        );
        if self.config.access == Access::ReadOnly {
            instructions.push_str(" This knowledge base is read-only.");
        } else {
            instructions.push_str(
                " A note you write is recorded as agent-written; a human approves it later.",
            );
        }
        if self.config.surfaces.has(Surface::Resources) && self.config.surfaces.has(Surface::Tools)
        {
            instructions
                .push_str(" Prefer reading zettel://note/ resources when you support them.");
        }

        InitializeResult::new(capabilities)
            .with_protocol_version(ProtocolVersion::default())
            .with_server_info({
                let mut info = Implementation::from_build_env();
                info.name.clone_from(&self.config.server_name);
                info.version = env!("CARGO_PKG_VERSION").to_string();
                info
            })
            .with_instructions(instructions)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, McpError> {
        if !self.config.surfaces.has(Surface::Tools) {
            return Ok(ListToolsResult::default());
        }
        Ok(ListToolsResult::with_all_items(self.tool_definitions()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, McpError> {
        if !self.config.surfaces.has(Surface::Tools) {
            return Err(McpError::invalid_request("tools are not served here", None));
        }
        let args = request.arguments.unwrap_or_default();
        match self.call(&request.name, &args) {
            Ok(text) => Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into()),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(e.to_string())]).into()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListResourcesResult, McpError> {
        if !self.config.surfaces.has(Surface::Resources) {
            return Ok(ListResourcesResult::default());
        }
        let ws = self.workspace().map_err(|e| to_mcp_error(&e))?;
        let aliases = ws.aliases();
        let resources = ws
            .all()
            .iter()
            .map(|v| {
                let mut resource = Resource::new(
                    Self::uri_of(&v.qualified, &aliases),
                    v.note.frontmatter.title.clone(),
                );
                resource.mime_type = Some("text/markdown".into());
                resource.description = Some(format!(
                    "provenance: {}",
                    v.note
                        .frontmatter
                        .provenance
                        .clone()
                        .unwrap_or_else(|| "unknown".into())
                ));
                resource
            })
            .collect();
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ReadResourceResponse, McpError> {
        if !self.config.surfaces.has(Surface::Resources) {
            return Err(McpError::invalid_request(
                "resources are not served here",
                None,
            ));
        }
        let id = self.note_id_of(&request.uri).map_err(|e| to_mcp_error(&e))?;
        let ws = self.workspace().map_err(|e| to_mcp_error(&e))?;
        let view = ws.find(&id).map_err(|e| to_mcp_error(&e))?;
        let text = pretty(&view.note.view(&view.qualified));
        Ok(ReadResourceResult::new(vec![ResourceContents::text(text, request.uri)]).into())
    }
}

/// Serve a knowledge base, on stdio or over HTTP.
pub fn run(config: ServeConfig, bind: Option<&str>) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(Error::Io)?;
    runtime.block_on(async move {
        match bind {
            None => serve_stdio(config).await,
            Some(addr) => serve_http(config, addr).await,
        }
    })
}

async fn serve_stdio(config: ServeConfig) -> Result<()> {
    use rmcp::ServiceExt;
    let service = ZettelServer::new(config)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
    service
        .waiting()
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
    Ok(())
}

async fn serve_http(config: ServeConfig, addr: &str) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    let read_only = config.access == Access::ReadOnly;
    let server = ZettelServer::new(config);
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default(),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(Error::Io)?;
    let bound = listener.local_addr().map_err(Error::Io)?;
    eprintln!(
        "zettel serving on http://{bound}/mcp ({})",
        if read_only { "read-only" } else { "read-write" }
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(Error::Io)?;
    Ok(())
}

/// Build a serve configuration from the command line.
pub fn config_from(
    root: &Path,
    surfaces: &str,
    access: &str,
    server_name: &str,
) -> Result<ServeConfig> {
    let surfaces =
        mdstore::mcp::Surfaces::parse_list(surfaces).map_err(crate::provenance::from_mdstore)?;
    let access: Access = access.parse().map_err(crate::provenance::from_mdstore)?;
    let mut config = ServeConfig::read_only(root, surfaces, server_name);
    config.access = access;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdstore::mcp::Surfaces;

    fn fixture() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "zettel-serve-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join(".zettel")).unwrap();
        std::fs::write(base.join("zettel.yml"), "zettel_dir: .zettel\n").unwrap();
        std::fs::write(
            base.join(".zettel/a1b2-pooling.md"),
            "---\ntitle: Pooling causes stale reads\nprovenance: human:dana\ntags:\n- db\nlinks: []\n---\n\nThe pool reuses connections.\n",
        )
        .unwrap();
        base
    }

    fn server(access: Access) -> ZettelServer {
        let mut config =
            ServeConfig::read_only(fixture(), Surfaces::resources_and_tools(), "zettel");
        config.access = access;
        ZettelServer::new(config)
    }

    #[test]
    fn a_read_only_server_offers_no_write_tool() {
        let names: Vec<String> = server(Access::ReadOnly)
            .tool_definitions()
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        assert!(names.contains(&"zettel_read_note".to_string()));
        assert!(!names.contains(&"zettel_create_note".to_string()));
    }

    #[test]
    fn a_writable_server_offers_the_write_tool() {
        let names: Vec<String> = server(Access::ReadWrite)
            .tool_definitions()
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        assert!(names.contains(&"zettel_create_note".to_string()));
    }

    #[test]
    fn approval_is_never_a_tool() {
        // Approving content is a human act. No access mode offers it.
        for access in [Access::ReadOnly, Access::ReadWrite] {
            let names: Vec<String> = server(access)
                .tool_definitions()
                .iter()
                .map(|t| t.name.to_string())
                .collect();
            assert!(
                !names.iter().any(|n| n.contains("review") || n.contains("approve")),
                "{names:?}"
            );
        }
    }

    #[test]
    fn a_read_only_server_refuses_a_write() {
        let s = server(Access::ReadOnly);
        let mut args = Map::new();
        args.insert("title".into(), json!("New note"));
        let err = s.call("zettel_create_note", &args).unwrap_err().to_string();
        assert!(err.contains("read-only"), "{err}");
    }

    #[test]
    fn the_server_stamps_agent_provenance_on_a_write() {
        let s = server(Access::ReadWrite);
        let mut args = Map::new();
        args.insert("title".into(), json!("Agent wrote this"));
        args.insert("body".into(), json!("Some derived text."));
        let out = s.call("zettel_create_note", &args).unwrap();
        assert!(out.contains("agent:summary"), "{out}");

        // What landed on disk says agent, without the caller asking.
        let ws = s.workspace().unwrap();
        let view = ws.find("agent-wrote-this").unwrap();
        assert_eq!(
            view.note.frontmatter.provenance.as_deref(),
            Some("agent:summary")
        );
    }

    #[test]
    fn a_caller_cannot_claim_human_provenance() {
        // The kind names the agent's claim, and only an agent kind is
        // accepted. There is no argument that yields `human`.
        let s = server(Access::ReadWrite);
        let mut args = Map::new();
        args.insert("title".into(), json!("Claimed"));
        args.insert("kind".into(), json!("human"));
        let err = s.call("zettel_create_note", &args).unwrap_err().to_string();
        assert!(err.contains("not an agent kind"), "{err}");
    }

    #[test]
    fn a_body_may_not_carry_provenance_markers() {
        // Without this, a caller writes its own spans: an inline human
        // span, or an agent span carrying a forged reviewed= stamp.
        // The note then matches --provenance human and --provenance
        // reviewed, and leaves the unreviewed queue.
        let s = server(Access::ReadWrite);
        for body in [
            "<!-- prov human:cody -->\nclaimed\n<!-- /prov -->",
            "<!-- prov agent:inference reviewed=2026-08-14 reviewer=cody -->\nx\n<!-- /prov -->",
            "text\n<!-- prov human -->\nmore",
        ] {
            let mut args = Map::new();
            args.insert("title".into(), json!("Forged"));
            args.insert("body".into(), json!(body));
            let err = s.call("zettel_create_note", &args).unwrap_err().to_string();
            assert!(err.contains("may not carry"), "{err} for {body}");
        }
    }

    #[test]
    fn a_plain_body_is_accepted() {
        let s = server(Access::ReadWrite);
        let mut args = Map::new();
        args.insert("title".into(), json!("Plain"));
        args.insert("body".into(), json!("Ordinary text with no markers."));
        assert!(s.call("zettel_create_note", &args).is_ok());
    }

    #[test]
    fn the_agent_kind_can_be_chosen_among_the_agent_kinds() {
        let s = server(Access::ReadWrite);
        let mut args = Map::new();
        args.insert("title".into(), json!("A guess"));
        args.insert("kind".into(), json!("inference"));
        let out = s.call("zettel_create_note", &args).unwrap();
        assert!(out.contains("agent:inference"), "{out}");
    }

    #[test]
    fn reading_a_note_carries_its_spans_and_provenance() {
        let s = server(Access::ReadOnly);
        let mut args = Map::new();
        args.insert("id".into(), json!("pooling"));
        let out = s.call("zettel_read_note", &args).unwrap();
        assert!(out.contains("human:dana"), "{out}");
        assert!(out.contains("spans"), "{out}");
    }

    #[test]
    fn a_note_uri_round_trips() {
        let s = server(Access::ReadOnly);
        let uri = ZettelServer::uri_of("a1b2-pooling", &[]);
        assert_eq!(uri, "zettel://note/a1b2-pooling");
        assert_eq!(s.note_id_of(&uri).unwrap(), "a1b2-pooling");
    }

    #[test]
    fn a_uri_of_another_scheme_is_refused() {
        let s = server(Access::ReadOnly);
        for bad in ["http://note/a1b2", "zettel://issue/a1b2", "nonsense"] {
            assert!(s.note_id_of(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn the_instructions_tell_a_reader_how_to_weigh_provenance() {
        let info = server(Access::ReadOnly).get_info();
        let instructions = info.instructions.unwrap_or_default();
        assert!(instructions.contains("provenance"), "{instructions}");
        assert!(instructions.contains("read-only"), "{instructions}");
    }
}
