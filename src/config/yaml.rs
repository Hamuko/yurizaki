use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub source: Option<String>,
    pub library: Option<String>,
    pub debounce: Option<u64>,

    #[serde(default)]
    pub trash: bool,

    #[serde(flatten)]
    pub rules: IndexMap<String, Rule>,
}

#[derive(Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub groups: Vec<String>,

    #[serde(default)]
    pub aliases: Vec<String>,

    #[serde(default)]
    pub minimum: RuleMinimum,

    #[cfg(feature = "regex")]
    #[serde(default)]
    pub regex: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RuleMinimum {
    #[serde(rename = "episode")]
    pub episode_number: Option<i64>,
}
