use std::borrow::BorrowMut;
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;

#[derive(Debug)]
enum MatchPart {
    Regex(regex::bytes::Regex),
    Plain(String),
}

#[derive(Debug)]
pub struct Pattern {
    parts: Vec<MatchPart>,
    raw: PathBuf,
}

pub struct Match {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub path: PathBuf,
}

/// By construction there are no path separators ("/") in the input.
fn regex_from_part(s: &[u8]) -> anyhow::Result<MatchPart> {
    struct MaybePattern {
        has_pattern: bool,
        out: Vec<u8>,
    }

    impl MaybePattern {
        fn append_pattern(&mut self, p: &[u8]) {
            self.out.extend_from_slice(p);
            self.has_pattern = true;
        }
        fn append_raw(&mut self, p: u8) {
            self.out.push(p);
        }
    }

    let mut local = MaybePattern {
        has_pattern: false,
        out: vec![],
    };
    let mut it = s.iter();

    while let Some(x) = it.next() {
        if *x == b'%' {
            match it.next() {
                Some(b'Y') => local.append_pattern(b"(?P<year4>\\d{4})"),
                Some(b'y') => local.append_pattern(b"(?P<year2>\\d{2})"),
                Some(b'm') => local.append_pattern(b"(?P<month>\\d{2})"),
                Some(b'd') => local.append_pattern(b"(?P<day>\\d{2})"),
                Some(b'H') => local.append_pattern(b"(?P<hour>\\d{2})"),
                Some(b'M') => local.append_pattern(b"(?P<minute>\\d{2})"),
                Some(b'S') => local.append_pattern(b"(?P<second>\\d{2})"),
                Some(b'%') => local.append_raw(*x),
                Some(y) => return Err(anyhow::Error::msg(format!("unsupported % escape: {}", y))),
                None => return Err(anyhow::Error::msg("incomplete % escape")),
            }
        } else {
            match *x {
                b'*' => local.append_pattern(b".*?"),
                b'.' => local.append_pattern(b"\\."),
                _ => local.append_raw(*x),
            }
        }
    }
    if local.has_pattern {
        Ok(MatchPart::Regex(regex::bytes::Regex::new(
            std::str::from_utf8(&local.out)?,
        )?))
    } else {
        Ok(MatchPart::Plain(
            (std::str::from_utf8(&local.out)?).to_owned(),
        ))
    }
}

impl Pattern {
    pub fn from_path(p: &std::path::Path) -> anyhow::Result<Self> {
        let mut parts = Vec::new();

        if !p.has_root() {
            return Err(anyhow::Error::msg(
                "Path must be absolute to avoid accidents due to working directory changes.",
            ));
        }

        for x in p.components() {
            match x {
                std::path::Component::Prefix(_) => unimplemented!("only supporting local unix FS"),
                std::path::Component::Normal(part) => {
                    parts.push(regex_from_part(part.as_bytes())?);
                }
                _ => {}
            }
        }
        Ok(Pattern {
            raw: p.to_owned(),
            parts,
        })
    }

    pub fn matches(&self) -> anyhow::Result<Vec<Match>> {
        let mut stack = std::collections::BTreeSet::from_iter(vec![PathBuf::from("/")]);

        for part in &self.parts {
            match part {
                MatchPart::Regex(part) => {
                    stack = stack
                        .iter()
                        .flat_map(|x| {
                            let mut out = vec![];
                            match std::fs::read_dir(&x) {
                                Ok(dirents) => {
                                    for ent in dirents {
                                        let mut x = x.clone();
                                        let name = ent.unwrap().file_name();
                                        if part.is_match(name.as_bytes()) {
                                            // TODO - keep track of matches for date construction
                                            part.captures_iter(name.as_bytes());
                                            x.push(name);
                                            out.push(x);
                                        }
                                    }
                                }
                                Err(_) => {
                                    // TOOD log more detailed warning
                                    println!("warning");
                                }
                            }
                            out
                        })
                        .collect();
                }
                MatchPart::Plain(part) => {
                    stack = stack
                        .iter()
                        .map(|x| {
                            let mut x = x.clone();
                            x.push(part);
                            x
                        })
                        .collect();
                }
            }
        }
        println!("{:#?}", stack);
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_construction() {
        super::Pattern::from_path(std::path::Path::new("/some/path/*.%Y-%m-%d")).unwrap();
        super::Pattern::from_path(std::path::Path::new("/some/path/%Y/%Y-%m-%d.log")).unwrap();
        super::Pattern::from_path(std::path::Path::new("/%%")).unwrap();

        super::Pattern::from_path(std::path::Path::new("/some/path/%Y/%Y-%m-%x.log")).unwrap_err();
    }

    #[test]
    fn test_matching() -> anyhow::Result<()> {
        let test = tempdir::TempDir::new("test")?;
        std::fs::File::create(test.path().join("rotated.2021-12-24.log"))?;
        std::fs::File::create(test.path().join("rotated.202x-12-24.log"))?;
        std::fs::File::create(test.path().join("rotated.2022-01-01.log"))?;

        let p = super::Pattern::from_path(&test.path().join("*.%Y-%m-%d.log"))?;
        let m = p.matches();


        Ok(())
    }
}
