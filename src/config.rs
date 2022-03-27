use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub enum TimeSource {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "btime")]
    BTime,
    #[serde(rename = "mtime")]
    MTime,
    #[serde(rename = "filename")]
    Filename,
}

fn default_auto() -> TimeSource {
    TimeSource::Auto
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(rename = "match", deserialize_with = "deserialize_pattern")]
    pub path_match: crate::matching::Pattern,
    #[serde(deserialize_with = "deserialize_duration")]
    pub after: chrono::Duration,
    pub run: Vec<String>,

    #[serde(default = "default_auto")]
    pub time_source: TimeSource,
}

fn deserialize_duration<'de, D>(d: D) -> Result<chrono::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    chrono::Duration::from_std(parse_duration::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)?)
        .map_err(serde::de::Error::custom)
}

fn deserialize_pattern<'de, D>(d: D) -> Result<crate::matching::Pattern, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::matching::Pattern::from_path(Path::new(&String::deserialize(d)?)).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rules: std::collections::BTreeMap<String, Rule>,
}

pub fn from_path(file_path: &Path) -> Result<Config> {
    let data = std::fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&data)?;
    Ok(config)
}


#[cfg(test)]
mod tests {
    #[test]
    fn load_examples() {
        let paths = std::fs::read_dir("./examples").unwrap();
        for x in paths {
            let path = x.unwrap().path();
            match path.extension() {
                Some(ext) if ext.to_string_lossy() == "toml" => {
                    super::from_path(&path).expect(&format!("could not parse {:?}", path));
                }
                _ => {}
            }

        }
    }
}