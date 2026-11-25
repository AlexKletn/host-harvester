use std::{
    collections::HashSet,
    fs, io,
    path::{Component, Path},
};

use hharvester_values::validate_key;
use serde::Deserialize;

use crate::Source;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub sources: Vec<Source>,
}

impl Profile {
    pub fn load(path: &Path) -> io::Result<Self> {
        let profile: Self = toml::from_str(&fs::read_to_string(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        profile.validate()?;
        Ok(profile)
    }

    fn validate(&self) -> io::Result<()> {
        let mut keys = HashSet::new();
        for source in &self.sources {
            if !source.path.is_absolute() || source.entries.is_empty() {
                return Err(invalid(
                    "each source needs an absolute path and at least one entry",
                ));
            }
            for entry in &source.entries {
                validate_key(&entry.key)?;
                if !keys.insert(&entry.key) {
                    return Err(invalid(format!("duplicate key: {}", entry.key)));
                }
                let mut parts = entry.path.components();
                if !matches!(parts.next(), Some(Component::Normal(_)))
                    || parts.any(|part| !matches!(part, Component::Normal(_)))
                {
                    return Err(invalid(format!("invalid relative path for {}", entry.key)));
                }
            }
        }
        if keys.is_empty() {
            return Err(invalid("profile has no entries"));
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
