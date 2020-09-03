use std::collections::HashMap;
use std::str::FromStr;

use web::db::{execute_sql, get_top_entries, read_and_add_entries};
use web::log::setup_logger;

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
use tide::http::{Mime, Response, StatusCode};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};
use tide::Request;

use r2d2::Pool;
use r2d2_sqlite::rusqlite::params;
use r2d2_sqlite::SqliteConnectionManager;

#[async_std::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    setup_logger()?;

    std::fs::remove_file("./blog.db").ok();
    info!("Deleted blog.db");
    let manager = SqliteConnectionManager::file("./blog.db");
    info!("Created blog.db");
    let pool = r2d2::Pool::new(manager).unwrap();
    execute_sql(
        &pool,
        "CREATE TABLE blog (cid INTEGER, title STRING, slug STRING, date STRING, content STRING)",
        params![],
    )?;
    info!("Created blog table.");

    let cid_map = read_and_add_entries(&pool)?;
    info!("Created blog hashmap.");

    let state = State {
        pool: Box::new(pool),
        cid_map,
    };

    let mut app = tide::with_state(state);

    // serve blog
    app.at("/")
        .get(|req: Request<State>| async move { root(req.state().pool.as_ref()).await });
    app.at("/blog/*blog_slug")
        .get(|req: Request<State>| async move { serve_entry(&req).await });
    // serve static
    // app.at("/static").get(|_| async { root().await });

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

#[derive(Clone)]
struct State {
    pool: Box<Pool<SqliteConnectionManager>>,
    cid_map: HashMap<String, usize>,
}

async fn root(
    pool: &Pool<SqliteConnectionManager>,
) -> std::result::Result<Response, tide::http::Error> {
    let entries = get_top_entries(pool).unwrap();
    let entry = entries.first().unwrap();
    let mime = Mime::from_str("text/html;charset=utf-8").unwrap();
    let mut response = Response::new(StatusCode::Ok);
    response.set_body(&entry.html[..]);
    response.set_content_type(mime);
    Ok(response)
}

async fn serve_entry(req: &Request<State>) -> std::result::Result<Response, tide::http::Error> {
    info!("Got entry request from {} -> {}",
        req.peer_addr().unwrap_or("unknown"),
        req.url().as_str(),
        );
    let pool = req.state().pool.as_ref();
    let cid_map = &req.state().cid_map;
    let slug: String = req.param("blog_slug")?;
    if let Some(cid) = cid_map.get(&slug) {
        let entry = web::db::get_entry_cid(pool, *cid).unwrap();
        let mime = Mime::from_str("text/html;charset=utf-8").unwrap();
        let mut response = Response::new(StatusCode::Ok);
        response.set_body(&entry.html[..]);
        response.set_content_type(mime);
        return Ok(response);
    }
    debug!("Tried to serve blog/{} but it does not exist.", slug);
    Err(tide::http::Error::from_str(
        tide::StatusCode::NotFound,
        format!("Blog {} does not exist.", slug),
    ))
}
