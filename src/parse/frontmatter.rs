use std::cmp::{Eq, PartialEq};
use std::path::PathBuf;

use serde_derive::{Serialize, Deserialize};

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};

#[derive(Serialize, Deserialize, Debug)]
pub struct FrontMatter {
    pub title: String,
    pub slug: String,
    pub date: String,
}

impl PartialEq for FrontMatter {
    fn eq(&self, other: &Self) -> bool {
        self.slug == other.slug && self.title == other.title
    }
}

impl Eq for FrontMatter {}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Encountered unexpected EOF while parsing frontmatter.")]
    EOF,
}

enum State {
    ReadingFrontMatter { buf: String, new_line: bool },
    ReadingMarker { count: usize },
    SkipNewline,
}

impl FrontMatter {
    pub fn parse(path: &PathBuf, input: &str) -> Result<(Self, usize)> {
        debug!("Parsing frontmatter for entry: {:?}", path);
        let mut state = State::ReadingFrontMatter {
            buf: String::new(),
            new_line: true,
        };

        let mut payload = None;
        let offset;

        let mut chars = input.char_indices();
        loop {
            let (idx, ch) = match chars.next() {
                Some(thing) => thing,
                _ => return Err(ParseError::EOF).wrap_err("Parse error:"),
            };
            match &mut state {
                State::ReadingFrontMatter { buf, new_line } => match ch {
                    '=' if *new_line => {
                        payload = Some(buf.to_owned());
                        state = State::ReadingMarker { count: 1 };
                    }
                    ch => {
                        buf.push(ch);
                        *new_line = ch == '\n';
                    }
                },
                State::ReadingMarker { count } => match ch {
                    '=' => {
                        *count += 1;
                        if *count == 3 {
                            state = State::SkipNewline;
                        }
                    }
                    marker => panic!("Expected marker, got {:?}", marker),
                },
                State::SkipNewline => match ch {
                    '\n' => {
                        offset = idx + 1;
                        break;
                    }
                    _ => panic!("Expected newline, got {:?}",),
                },
            }
        }
        let frontmatter = toml::from_str(&payload.unwrap())
            .map_err(|e| Report::from(e).wrap_err(format!("while parsing {:?}", path)))?;
        Ok((frontmatter, offset))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use indoc::indoc;

    fn example_frontmatter() -> FrontMatter {
        toml::from_str(
            r#"
            slug = "/a/b"
            title = "Test"
            date = "2020-09-03 07:48:00"
            "#,
        )
        .unwrap()
    }

    #[test]
    fn deserialize_page_config() {
        let frontmatter = example_frontmatter();
        assert_eq!(frontmatter.slug, "/a/b");
        assert_eq!(frontmatter.title, "Test");
        assert_eq!(frontmatter.date, "2020-09-03 07:48:00");
    }

    #[test]
    fn valid_parse() {
        let expected_frontmatter = example_frontmatter();
        let content = indoc! {r#"
            slug = "/a/b"
            title = "Test"
            date = "2020-09-03 07:48:00"
            ===
            Markdown begins
        "#};
        let (frontmatter, offset) = FrontMatter::parse(&"test".into(), &content).unwrap();
        assert_eq!(expected_frontmatter, frontmatter);
        assert_eq!(&content[offset..], "Markdown begins\n")
    }
}
