use std::path::Path;

use anyhow::Result;
use argh::FromArgs;
use serde::Deserialize;

#[derive(argh::FromArgs)]
/// lifecycled - life cycle daemon
struct Args {
    /// path to config file in TOML format
    #[argh(option)]
    config: std::path::PathBuf,
}

pub mod matching;

#[derive(Debug, Deserialize)]
struct Rule {
    #[serde(rename = "match", deserialize_with = "deserialize_pattern")]
    path_match: matching::Pattern,
    #[serde(deserialize_with = "deserialize_duration")]
    after: chrono::Duration,
    run: Vec<String>,
}

fn deserialize_duration<'de, D>(d: D) -> Result<chrono::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    chrono::Duration::from_std(parse_duration::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)?)
        .map_err(serde::de::Error::custom)
}

fn deserialize_pattern<'de, D>(d: D) -> Result<matching::Pattern, D::Error>
where
    D: serde::Deserializer<'de>,
{
    matching::Pattern::from_path(Path::new(&String::deserialize(d)?)).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct Config {
    rules: std::collections::BTreeMap<String, Rule>,
}

fn from_path(file_path: &Path) -> Result<Config> {
    let data = std::fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&data)?;
    Ok(config)
}

#[derive(Debug)]
struct RuleApplication {
    path: std::path::PathBuf,
    commands: Vec<String>,
}

// Filters down to the matches where the time condition applies.
fn step(config: &Config, now: chrono::NaiveDateTime) -> anyhow::Result<Vec<RuleApplication>> {
    let mut out = vec![];
    for (name, rule) in &config.rules {
        for m in rule.path_match.matches()? {
            if m.timestamp
                .checked_add_signed(rule.after)
                .ok_or_else(|| anyhow::Error::msg("time addition failed"))?
                <= now
            {
                // TODO(tom): make rule an Arc for sharing
                out.push(RuleApplication {
                    path: m.path,
                    commands: rule.run.clone(),
                });
            }
        }
    }

    Ok(out)
}

fn main() {
    env_logger::init();
    let args: Args = argh::from_env();
    let config = from_path(&args.config).unwrap();
    log::debug!("Config: {:?}", config);
    loop {
        // evaluate once a minute, should be enough
        match step(&config, chrono::Utc::now().naive_utc()) {
            Ok(applications) => {
                log::debug!("Evaluation returned {} steps: {:?}", applications.len(), applications);
                for x in applications {
                    for cmd in x.commands {
                        match std::process::Command::new("bash")
                            .env("LIFECYCLED_PATH", x.path.as_os_str())
                            .arg("-c")
                            .arg(&cmd)
                            .spawn()
                        {
                            Ok(ref mut process) => {process.wait();},
                            Err(err) => log::warn!("Command error {}: {:?}", cmd, err),
                        }
                    }
                }
            }
            Err(err) => log::warn!("Evaluation error: {:?}", err),
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_step() -> anyhow::Result<()> {
        // let now = chrono::Utc::now().naive_utc();
        let test = tempdir::TempDir::new("test")?;
        std::fs::File::create(test.path().join("rotated.2020-11-13.log"))?;

        let config = Config {
            rules: std::collections::BTreeMap::from_iter(vec![(
                "test-rule".into(),
                Rule {
                    path_match: matching::Pattern::from_path(&test.path().join("*.%Y-%m-%d"))?,
                    after: chrono::Duration::days(1),
                    run: vec!["cat".to_owned()],
                },
            )]),
        };

        let applications = step(&config, chrono::NaiveDate::from_ymd(2020, 11, 13).and_hms(0, 0, 0))?;
        assert_eq!(applications.len(), 0);

        let applications = step(&config, chrono::NaiveDate::from_ymd(2020, 11, 14).and_hms(0, 0, 0))?;
        assert_eq!(applications.len(), 1);

        assert_eq!(applications[0].commands, vec!["cat"]);

        Ok(())
    }
}
