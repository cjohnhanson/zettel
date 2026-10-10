#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a zettel repository (zettel.yml not found)")]
    NotInitialized,

    #[error("already initialized (zettel.yml exists)")]
    AlreadyInitialized,

    #[error("note '{0}' not found")]
    NoteNotFound(String),

    #[error("ambiguous prefix '{0}'; it matches several notes")]
    AmbiguousPrefix(String),

    #[error("note '{0}' already exists")]
    NoteAlreadyExists(String),

    #[error("invalid provenance: {0}")]
    InvalidProvenance(String),

    #[error("store: {0}")]
    Store(String),

    #[error("{tool} needs '{arg}'")]
    MissingArgument { tool: String, arg: String },

    #[error("no tool named '{0}'")]
    UnknownTool(String),

    #[error("span {0} is not an agent span; review applies to agent spans")]
    SpanNotAgent(usize),

    #[error("no span {0}; run 'zettel note review <id>' to list the spans")]
    SpanNotFound(usize),

    #[error("invalid selector '{0}'; use tag:<value>, provenance:<token>, or link:<id>")]
    InvalidSelector(String),

    #[error("store '{0}' is not declared in stores.yml")]
    UndeclaredStore(String),

    #[error(
        "'{0}' is in store '{1}'; a dependency store is read-only. Run the command from that store to edit the note"
    )]
    ForeignWrite(String, String),

    #[error("unknown field '{0}'")]
    UnknownField(String),

    #[error("the record has no string value for the key field '{0}'")]
    MissingKey(String),

    #[error(
        "the key value '{value}' is held by more than one note: {}. Remove the key from all but one",
        ids.join(", ")
    )]
    DuplicateKey { value: String, ids: Vec<String> },

    #[error("zettel owns the field name '{0}'; use another name")]
    ReservedField(String),

    #[error("a field name must be non-empty text, not {0}")]
    InvalidFieldName(String),

    #[error("note '{id}' cannot be read: {reason}. Fix or remove the note, then run again")]
    UnreadableNote { id: String, reason: String },

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Yaml(#[from] yaml_serde::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
