pub mod cli;
pub mod config;
pub mod error;
pub mod note;
pub mod repo;

pub use config::ZettelConfig;
pub use error::{Error, Result};
pub use note::{Note, NoteFrontmatter};
pub use repo::{CreateNoteOptions, EditNoteOptions, Repo};
