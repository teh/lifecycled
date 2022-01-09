use std::path::Path;

#[derive(argh::FromArgs)]
/// lifecycled - life cycle daemon
struct Args {
    /// path to config file in TOML format
    #[argh(option)]
    config: std::path::PathBuf,

    /// dry run  to see what would happen
    #[argh(switch)]
    dry_run: bool,
}

pub mod config;
pub mod matching;

#[derive(Debug)]
struct RuleApplication {
    path: std::path::PathBuf,
    commands: Vec<String>,
}

fn mtime(path: &Path) -> chrono::NaiveDateTime {
    match std::fs::metadata(path) {
        Ok(metadata) => chrono::DateTime::<chrono::Utc>::from(metadata.modified().unwrap()).naive_utc(),
        Err(_) => chrono::naive::MAX_DATETIME,
    }
}

fn ctime(path: &Path) -> chrono::NaiveDateTime {
    match std::fs::metadata(path) {
        Ok(metadata) => chrono::DateTime::<chrono::Utc>::from(metadata.created().unwrap()).naive_utc(),
        Err(_) => chrono::naive::MAX_DATETIME,
    }
}

// Filters down to the matches where the time condition applies.
fn step(config: &config::Config, now: chrono::NaiveDateTime) -> anyhow::Result<Vec<RuleApplication>> {
    let mut out = vec![];
    for (name, rule) in &config.rules {
        for m in rule.path_match.matches()? {
            let timestamp = match rule.time_source {
                config::TimeSource::Auto if m.timestamp.is_none() => mtime(&m.path),
                config::TimeSource::Auto if m.timestamp.is_some() => m.timestamp.unwrap(),
                config::TimeSource::CTime => ctime(&m.path),
                config::TimeSource::MTime => mtime(&m.path),
                config::TimeSource::Filename if m.timestamp.is_some() => m.timestamp.unwrap(),
                _ => {
                    log::warn!("could not derive timestamp for {:?}", m.path);
                    chrono::naive::MAX_DATETIME
                }
            };

            if timestamp
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

fn dry_run(config: &config::Config) {
    let mut ts = chrono::Utc::now().naive_utc();
    loop {
        ts = ts.checked_add_signed(chrono::Duration::minutes(1)).unwrap();

        match step(config, ts) {
            Ok(applications) => {
                log::debug!("Evaluation returned {} steps: {:?}", applications.len(), applications);
                for x in applications {
                    println!("[{:?}] Match {:?}, run {:?}", ts, x.path, x.commands);
                }
            }
            Err(_) => todo!(),
        }
    }
}

fn main() {
    env_logger::init();
    let args: Args = argh::from_env();
    let config = config::from_path(&args.config).unwrap();
    log::debug!("Config: {:?}", config);

    if args.dry_run {
        dry_run(&config);
    } else {
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
                                Ok(ref mut process) => {
                                    process.wait();
                                }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_step() -> anyhow::Result<()> {
        // let now = chrono::Utc::now().naive_utc();
        let test = tempdir::TempDir::new("test")?;
        std::fs::File::create(test.path().join("rotated.2020-11-13.log"))?;

        let config = Config {
            rules: std::collections::BTreeMap::from_iter(vec![(
                "test-rule".into(),
                Rule {
                    path_match: matching::Pattern::from_path(&test.path().join("*.%Y-%m-%d.log"))?,
                    after: chrono::Duration::days(1),
                    run: vec!["cat".to_owned()],
                    time_source: TimeSource::Auto,
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
