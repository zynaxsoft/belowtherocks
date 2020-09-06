use std::collections::HashMap;
use std::path::PathBuf;

use web::db::{execute_sql, get_top_entries, read_and_add_entries};
use web::log::setup_logger;

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
use tide::http::{mime, StatusCode};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};
use tide::{Request, Response};

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
    // app.at("/")
    //     .get(|req: Request<State>| async move { root(req.state().pool.as_ref()).await });
    app.at("/blog/*blog_slug")
        .get(|req: Request<State>| async move { serve_entry(&req).await });
    app.at("/static/*file_path")
        .get(|req: Request<State>| async move { serve_static(&req).await });

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

#[derive(Clone)]
struct State {
    pool: Box<Pool<SqliteConnectionManager>>,
    cid_map: HashMap<String, usize>,
}

type TideResult = std::result::Result<Response, tide::http::Error>;

// async fn root(pool: &Pool<SqliteConnectionManager>) -> TideResult {
//     let entries = get_top_entries(pool).unwrap();
//     let entry = entries.first().unwrap();
//     response.set_body(&entry.html[..]);
//     response.set_content_type(mime);
//     Ok(response)
// }

async fn serve_static(req: &Request<State>) -> TideResult {
    let path: PathBuf = req.param("file_path")?;
    let extension = path.extension().unwrap().to_string_lossy().to_owned();
    let mime = match extension.as_ref() {
        "css" => mime::CSS,
        "jpeg" => mime::JPEG,
        "png" => mime::PNG,
        "svg" => mime::SVG,
        "js" => mime::JAVASCRIPT,
        "wasm" => mime::WASM,
        _ => mime::BYTE_STREAM,
    };
    let actual_path = PathBuf::from("./static").join(path);
    if !actual_path.is_file() {
        info!(
            "The requested file {:?} is not a file or doesn't exist!",
            actual_path
        );
        return Err(tide::http::Error::from_str(
            tide::StatusCode::NotFound,
            format!("{:?} is not a file or doesn't exist.", actual_path),
        ))
    }
    let body = tide::Body::from_bytes(std::fs::read(actual_path).unwrap());
    let response = Response::builder(200).content_type(mime).body(body).build();
    Ok(response)
}

async fn serve_entry(req: &Request<State>) -> TideResult {
    info!(
        "Got entry request from {} -> {}",
        req.peer_addr().unwrap_or("unknown"),
        req.url().as_str(),
    );
    let pool = req.state().pool.as_ref();
    let cid_map = &req.state().cid_map;
    let slug: String = req.param("blog_slug")?;
    if let Some(cid) = cid_map.get(&slug) {
        let entry = web::db::get_entry_cid(pool, *cid).unwrap();
        let mime = mime::HTML;
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