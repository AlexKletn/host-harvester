use std::path::PathBuf;

use hharvester_values::ValueKind;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub key: String,
    pub path: PathBuf,
    pub kind: ValueKind,
}
