use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    any,
    path::{Path, PathBuf},
};

pub mod matching;

#[derive(Debug, Deserialize)]
struct Rule {
    #[serde(rename = "match", deserialize_with = "deserialize_pattern")]
    path_match: matching::Pattern,
    #[serde(deserialize_with = "deserialize_duration")]
    after: std::time::Duration,
    run: Vec<String>,
}

fn deserialize_duration<'de, D>(d: D) -> Result<std::time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    parse_duration::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
}

fn deserialize_pattern<'de, D>(d: D) -> Result<matching::Pattern, D::Error>
where
    D: serde::Deserializer<'de>,
{
    matching::Pattern::from_path(Path::new(&String::deserialize(d)?)).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct Config {
    rules: std::collections::BTreeMap<String, Rule>,
}

pub fn from_path(file_path: &Path) -> Result<Config> {
    let data = std::fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&data)?;
    Ok(config)
}


fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = from_path(Path::new("./examples/log_files.toml")).unwrap();
    println!("config: {:#?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {

}
