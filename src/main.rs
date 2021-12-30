use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    any,
    path::{Path, PathBuf},
};

pub mod matching;

#[derive(Debug, Deserialize)]
struct Rule {
    #[serde(rename = "match")]
    path_match: String,
    #[serde(deserialize_with = "deserialize_duration")]
    after: std::time::Duration,
    run: Vec<String>,
}

pub fn deserialize_duration<'de, D>(d: D) -> Result<std::time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    parse_duration::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
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

fn glob_from_strptime_pattern(pattern: &str) -> String {
    enum State {
        Char,
        Escape,
    }
    let mut state = State::Char;
    let out: String = pattern
        .chars()
        .flat_map(|x| match state {
            State::Char => match x {
                '%' => {
                    state = State::Escape;
                    vec![]
                }
                _ => vec![x],
            },
            State::Escape => {
                state = State::Char;
                match x {
                    '%' => vec!['%'],
                    'Y' => "[0-9][0-9][0-9][0-9]".chars().collect(),
                    'y' => "[0-9][0-9]".chars().collect(),
                    'm' => "[0-9][0-9]".chars().collect(),
                    'd' => "[0-9][0-9]".chars().collect(),
                    'H' => "[0-9][0-9]".chars().collect(),
                    'M' => "[0-9][0-9]".chars().collect(),
                    'S' => "[0-9][0-9]".chars().collect(),
                    default => todo!("Pattern {} not implemented", default),
                }
            }
        })
        .collect();
    out
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
