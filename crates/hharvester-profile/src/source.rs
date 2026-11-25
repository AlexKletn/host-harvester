use std::path::PathBuf;

use serde::Deserialize;

use crate::Entry;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: PathBuf,
    pub filesystem: Option<String>,
    pub entries: Vec<Entry>,
}
