/// The name a user types, which is also the directory holding this
/// tool's user config and registry. One home, so the config path and
/// every message that names it cannot drift apart.
pub const TOOL: mdstore::ToolName<'static> = match mdstore::ToolName::new("zettel") {
    Some(t) => t,
    None => panic!("the tool name must be one plain path component"),
};

pub mod cli;
pub mod config;
pub mod error;
pub mod mangen;
pub mod note;
pub mod provenance;
pub mod repo;
pub mod selector;
pub mod serve;
pub mod workspace;

pub use config::ZettelConfig;
pub use error::{Error, Result};
pub use note::{Note, NoteFrontmatter};
pub use repo::{
    BrokenLink, CreateNoteOptions, EditNoteOptions, ListNotesFilter, MigrateAction, Repo,
    SearchResult, SpanCounts, Stats,
};
pub use selector::Selector;

#[cfg(test)]
mod tool_name_tests {
    /// The const is the directory this tool reads its config from. A
    /// wrong name reads another tool's file and fails nothing, because
    /// no test reaches `config_path` with the real name: the end-to-end
    /// wrapper pins --user-config and the registry is set from the
    /// environment. So bind the name to something that cannot drift.
    ///
    /// Not the package name. The package publishes as a name that was
    /// free on the registries, and the config directory must not follow
    /// that rename: it would move every reader's config out from under
    /// them. The command a person types is the right anchor, and
    /// Cargo.toml names it.
    #[test]
    fn the_tool_name_is_the_command_name() {
        let manifest = include_str!("../Cargo.toml");
        let mut lines = manifest.lines();
        let bin = loop {
            let Some(line) = lines.next() else {
                panic!("Cargo.toml declares no [[bin]]");
            };
            if line.trim() == "[[bin]]" {
                let name = lines
                    .find(|l| l.trim_start().starts_with("name = "))
                    .expect("a [[bin]] carries a name");
                break name.split('"').nth(1).expect("a quoted name").to_string();
            }
        };
        assert_eq!(super::TOOL.as_str(), bin);
        assert_eq!(super::TOOL.as_str(), "zettel");
    }
}
