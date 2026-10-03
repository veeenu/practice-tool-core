//! Building blocks for the tools' configuration files.

use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;

use crate::key::Key;

/// Either a value, or a placeholder `true` standing for no value.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PlaceholderOption<T> {
    Data(T),
    Placeholder(bool),
}

impl<T> PlaceholderOption<T> {
    pub fn into_option(self) -> Option<T> {
        match self {
            PlaceholderOption::Data(d) => Some(d),
            PlaceholderOption::Placeholder(_) => None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct LevelFilterSerde(LevelFilter);

impl LevelFilterSerde {
    pub fn inner(&self) -> LevelFilter {
        self.0
    }
}

impl TryFrom<String> for LevelFilterSerde {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(LevelFilterSerde(
            LevelFilter::from_str(&value)
                .map_err(|e| format!("Couldn't parse log level filter: {}", e))?,
        ))
    }
}

/// Entry of the radial menu.
#[derive(Debug, Deserialize, Clone)]
pub struct RadialMenu {
    pub key: Key,
    pub label: String,
}

impl AsRef<str> for RadialMenu {
    fn as_ref(&self) -> &str {
        &self.label
    }
}

/// Parses a TOML configuration, reporting where in it any error is.
pub fn parse_toml<T: DeserializeOwned>(s: &str) -> Result<T, String> {
    let de = toml::de::Deserializer::new(s);
    serde_path_to_error::deserialize(de)
        .map_err(|e| format!("TOML config error at {}: {}", e.path(), e.inner()))
}
