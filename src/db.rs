use std::collections::HashMap;

#[allow(unused_imports)]
use color_eyre::{
    eyre::WrapErr,
    eyre::{eyre, Report},
    Result, Section,
};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};

use r2d2::Pool;
use r2d2_sqlite::rusqlite::{params, ToSql};
use r2d2_sqlite::SqliteConnectionManager;

use crate::parse::{frontmatter::FrontMatter, get_target_path, parse_file, Entry};

pub fn read_and_add_entries(
    pool: &Pool<SqliteConnectionManager>,
) -> Result<HashMap<String, usize>> {
    let mut cid_map = HashMap::new();
    let paths = get_target_path(&"./blog".into())?;
    debug!("Trying to parse the gathered {} paths.", paths.len());
    for (cid, path) in paths.iter().enumerate() {
        let entry = parse_file(path)?;
        execute_sql(
            pool,
            "INSERT INTO blog (cid, title, slug, date, content) VALUES (?, ?, ?, ?, ?)",
            params![
                cid as i64,
                entry.fm.title,
                entry.fm.slug,
                entry.fm.date,
                entry.html
            ],
        )?;
        debug!("Mapping slug: {} with cid: {}", entry.fm.slug, cid);
        cid_map.insert(entry.fm.slug, cid);
    }
    Ok(cid_map)
}

pub fn execute_sql<T>(pool: &Pool<SqliteConnectionManager>, query: &str, param: T) -> Result<()>
where
    T: IntoIterator,
    T::Item: ToSql,
{
    pool.get()?.execute(query, param)?;
    Ok(())
}

pub fn get_top_entries(pool: &Pool<SqliteConnectionManager>) -> Result<Vec<Entry>> {
    let conn = pool.get()?;
    let query = "SELECT title, slug, date, content FROM blog";
    let mut stmt = conn.prepare(query)?;
    let result = stmt
        .query_map(params![], |row| {
            let fm = FrontMatter {
                title: row.get(0)?,
                slug: row.get(1)?,
                date: row.get(2)?,
            };
            Ok(Entry {
                fm,
                html: row.get(3)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect();
    Ok(result)
}

pub fn get_entry_cid(pool: &Pool<SqliteConnectionManager>, cid: usize) -> Result<Entry> {
    let conn = pool.get()?;
    let query = "SELECT title, slug, date, content FROM blog WHERE cid=?1";
    let mut stmt = conn.prepare(query)?;
    let result: Vec<Entry> = stmt
        .query_map(params![cid as i64], |row| {
            let fm = FrontMatter {
                title: row.get(0)?,
                slug: row.get(1)?,
                date: row.get(2)?,
            };
            Ok(Entry {
                fm,
                html: row.get(3)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect();
    let entry = result.into_iter().next().unwrap();
    debug!(
        "Got title: {}, slug: {}, date: {} with cid: {}",
        entry.fm.title, entry.fm.slug, entry.fm.date, cid
    );
    Ok(entry)
}
