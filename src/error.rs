#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a zettel repository (zettel.yml not found)")]
    NotInitialized,

    #[error("already initialized (zettel.yml exists)")]
    AlreadyInitialized,

    #[error("note '{0}' not found")]
    NoteNotFound(String),

    #[error("ambiguous prefix '{0}' — matches multiple notes")]
    AmbiguousPrefix(String),

    #[error("note '{0}' already exists")]
    NoteAlreadyExists(String),

    #[error("invalid provenance: {0}")]
    InvalidProvenance(String),

    #[error("span {0} is not an agent span; review applies to agent spans")]
    SpanNotAgent(usize),

    #[error("no span {0}; run 'zettel note review <id>' to list the spans")]
    SpanNotFound(usize),

    #[error("invalid selector '{0}'; use tag:<value>, provenance:<token>, or link:<id>")]
    InvalidSelector(String),

    #[error("unknown field '{0}'")]
    UnknownField(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Yaml(#[from] serde_yml::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
