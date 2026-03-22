pub mod cli;
pub mod config;
pub mod error;
pub mod note;
pub mod repo;

pub use config::ZettelConfig;
pub use error::{Error, Result};
pub use note::{Note, NoteFrontmatter, Status};
pub use repo::{CreateNoteOptions, EditNoteOptions, ListNotesFilter, Repo, SearchResult, Stats};
