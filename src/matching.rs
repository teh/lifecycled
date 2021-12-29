use std::{os::unix::prelude::OsStrExt, path::PathBuf};

#[derive(Debug)]
pub struct Pattern {
    parts: Vec<regex::bytes::Regex>,
    raw: PathBuf,
}

/// By construction there are no path separators ("/") in the input.
fn regex_from_part(s: &[u8]) -> anyhow::Result<regex::bytes::Regex> {
    let mut out: Vec<u8> = vec![];
    let mut it = s.iter();

    while let Some(x) = it.next() {
        if *x == b'%' {
            match it.next() {
                Some(b'Y') => out.extend_from_slice(b"(?P<year4>\\d{4})"),
                Some(b'y') => out.extend_from_slice(b"(?P<year2>\\d{2})"),
                Some(b'm') => out.extend_from_slice(b"(?P<month>\\d{2})"),
                Some(b'd') => out.extend_from_slice(b"(?P<day>\\d{2})"),
                Some(b'H') => out.extend_from_slice(b"(?P<hour>\\d{2})"),
                Some(b'M') => out.extend_from_slice(b"(?P<minute>\\d{2})"),
                Some(b'S') => out.extend_from_slice(b"(?P<second>\\d{2})"),
                Some(b'%') => out.push(*x),
                Some(y) => return Err(anyhow::Error::msg(format!("unsupported % escape: {}", y))),
                None => return Err(anyhow::Error::msg("incomplete % escape")),
            }
        } else {
            match *x {
                b'*' => out.extend_from_slice(b".*?"),
                b'.' => out.extend_from_slice(b"\\."),
                _ => out.push(*x),
            }
        }
    }
    Ok(regex::bytes::Regex::new(std::str::from_utf8(&out)?)?)
}

impl Pattern {
    pub fn from_path(p: &std::path::Path) -> anyhow::Result<Self> {
        let mut parts = Vec::new();
        for x in p.components() {
            match x {
                std::path::Component::Prefix(_) => unimplemented!("only supporting local unix FS"),
                std::path::Component::Normal(part) => {
                    // TODO (tom) - compile part to regex
                    part.as_bytes();
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
}
