use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    File,
    Text,
}

pub const MAX_TEXT_BYTES: usize = 65_535;
