use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ZettelConfig {
    pub zettel_dir: String,
}

impl Default for ZettelConfig {
    fn default() -> Self {
        Self {
            zettel_dir: ".zettel".into(),
        }
    }
}
