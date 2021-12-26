use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

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

impl Rule {
    fn eval(&self) -> Result<Vec<PathBuf>> {
        // We first use glob matching to generate a list of candidates and
        // then feed the list to strptime.
        let (matched, failed): (Vec<_>, Vec<_>) = glob::glob(&self.path_match)
            .with_context(|| "Failed to read glob pattern")?
            .partition(|x| x.is_ok());

        let matched: Result<Vec<PathBuf>, glob::GlobError> = matched.into_iter().collect();

        if !failed.is_empty() {
            // TODO: log warning
        }

        for m in matched? {
            match chrono::DateTime::parse_from_str(&m.to_string_lossy(), &self.path_match) {
                Ok(_) => todo!(),
                Err(_) => todo!(),
            }
        }
        Ok(vec![])
    }
}

fn main() -> std::io::Result<()> {
    let config = from_path(Path::new("./examples/log_files.toml")).unwrap();
    println!("config: {:#?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn single_rule() {
        let test = tempdir::TempDir::new("test").unwrap();
        std::fs::File::create(test.path().join("rotated.2021-12-24.log")).unwrap();
        std::fs::File::create(test.path().join("rotated.202x-12-24.log")).unwrap();

        let rule = super::Rule {
            path_match: test.path().join("*.%Y-%m-%d.log").to_string_lossy().into(),
            after: std::time::Duration::from_secs(10),
            run: vec!["ls $$".into()],
        };
        let r = rule.eval().unwrap();
        assert_eq!(r, vec![test.path().join("rotated.2021-12-24.log")])
    }
}
