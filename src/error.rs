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

    #[error("'{status}' is not a valid status")]
    InvalidStatus { status: String },

    #[error("unknown field '{0}'")]
    UnknownField(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Yaml(#[from] serde_yml::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
