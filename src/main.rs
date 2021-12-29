use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    any,
    path::{Path, PathBuf},
};

mod matching;

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

impl Rule {
    fn eval(&self) -> Result<Vec<PathBuf>> {
        // We first use glob matching to generate a list of candidates and
        // then feed the list to strptime.
        let pattern: String = glob_from_strptime_pattern(&self.path_match);

        // map error to remove bad lifetime
        let glob: wax::Glob =
            wax::Glob::new(&pattern).map_err(|x| anyhow::Error::msg(x.to_string()))?;
        let (candidates, failed): (Vec<_>, Vec<_>) = glob
            .walk(Path::new("/"), usize::MAX)
            .partition(|x| x.is_ok());

        let candidates = candidates
            .into_iter()
            .collect::<Result<Vec<wax::WalkEntry>, wax::GlobError>>()?;

        if !failed.is_empty() {
            // TODO: log warning
        }

        let matched: Vec<PathBuf> = candidates
            .into_iter()
            .flat_map(|x| {
                match chrono::DateTime::parse_from_str(
                    &x.path().to_string_lossy(),
                    &self.path_match,
                ) {
                    Ok(_) => vec![x.into_path()],
                    Err(e) => panic!(
                        "{}, {}, {:?}",
                        &x.path().to_string_lossy(),
                        &self.path_match,
                        e
                    ), //vec![],
                }
            })
            .collect();
        Ok(matched)
    }
}

fn main() -> std::io::Result<()> {
    env_logger::init();

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
