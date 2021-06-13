use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[allow(unused_imports)]
use color_eyre::{
    eyre::WrapErr,
    eyre::{eyre, Report},
    Result, Section,
};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};

pub mod frontmatter;
pub mod markdown;

#[derive(tide::convert::Serialize)]
pub struct Entry {
    pub fm: frontmatter::FrontMatter,
    pub html: String,
}

impl Entry {
    pub fn get_preview(&self) -> &str {
        self.html.lines().next().unwrap()
    }
}

pub fn parse_file(file_path: &PathBuf) -> Result<Entry> {
    let mut input = String::new();
    File::open(&file_path)?.read_to_string(&mut input)?;
    let (fm, offset) = frontmatter::FrontMatter::parse(&file_path, &input)?;
    let mut output = Vec::new();
    markdown::parse_md(&input[offset..], &mut output)?;
    debug!("Parsed {:?}", file_path);
    Ok(Entry {
        fm,
        html: String::from_utf8_lossy(&output).into_owned(),
    })
}

pub fn get_target_path(path: &PathBuf) -> Result<Vec<PathBuf>> {
    debug!("Gathering md file paths in {:?}", path);
    let mut result = Vec::new();
    if path.is_dir() {
        for dir_entry in std::fs::read_dir(path)? {
            let p = dir_entry.unwrap().path();
            if p.is_dir() {
                result.extend(get_target_path(&p)?);
            } else if let Some(extension) = p.extension() {
                if extension == "md" {
                    result.push(p);
                }
            }
        }
        return Ok(result);
    }
    Err(eyre!("{} is not a directory.", path.to_string_lossy()))
}
