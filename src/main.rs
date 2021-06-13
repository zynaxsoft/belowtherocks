use std::collections::HashMap;
use std::path::PathBuf;

use kore::db;
use kore::log::setup_logger;

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
use tide::http::mime;
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};
use tide::{Request, Response};

use r2d2::Pool;
use r2d2_sqlite::rusqlite::params;
use r2d2_sqlite::SqliteConnectionManager;

const ENTRIES_PER_PAGE: isize = 2;

#[async_std::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    setup_logger()?;

    std::fs::remove_file("./blog.db").ok();
    info!("Deleted blog.db");
    let manager = SqliteConnectionManager::file("./blog.db");
    info!("Created blog.db");
    let pool = r2d2::Pool::new(manager).unwrap();
    db::execute_sql(
        &pool,
        "CREATE TABLE blog (cid INTEGER, title STRING, slug STRING, date STRING, content STRING)",
        params![],
    )?;
    info!("Created blog table.");

    let cid_map = db::read_and_add_entries(&pool)?;
    info!("Created blog hashmap.");

    let liquid_parser = liquid::ParserBuilder::with_stdlib().build().unwrap();

    let state = State {
        pool: Box::new(pool),
        cid_map,
        liquid_parser: Box::new(liquid_parser),
    };

    let mut app = tide::with_state(state);

    // serve blog
    app.at("/")
        .get(|req: Request<State>| async move { serve_page_inner(&req, 0).await });
    app.at("/page/*page")
        .get(|req: Request<State>| async move { serve_page(&req).await });
    app.at("/blog/*blog_slug")
        .get(|req: Request<State>| async move { serve_entry(&req).await });
    app.at("/static/*file_path")
        .get(|req: Request<State>| async move { serve_static(&req).await });

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}

#[derive(Clone)]
struct State {
    pool: Box<Pool<SqliteConnectionManager>>,
    cid_map: HashMap<String, usize>,
    liquid_parser: Box<liquid::Parser>,
}

type TideResult = std::result::Result<Response, tide::http::Error>;

async fn serve_page(req: &Request<State>) -> TideResult {
    let page: isize = req.param("page")?.parse::<_>()?;
    serve_page_inner(req, page).await
}

async fn serve_page_inner(req: &Request<State>, page: isize) -> TideResult {
    let offset = page * ENTRIES_PER_PAGE;
    let entries = db::get_entries(req.state().pool.as_ref(), offset, ENTRIES_PER_PAGE).unwrap();

    let liquid = req.state().liquid_parser.as_ref();
    let template = std::fs::read_to_string("./templates/index.html.liquid").unwrap();
    let template = liquid.parse(&template).unwrap();

    #[derive(tide::convert::Serialize)]
    struct EntryWithPrev {
        preview: String,
        entry: kore::parse::Entry,
    }

    let entries: Vec<EntryWithPrev> = entries
        .into_iter()
        .map(|e| EntryWithPrev {
            preview: e.get_preview().to_string(),
            entry: e,
        })
        .collect();

    let params = liquid::object!({ "entries": entries, "page": page });
    let body = template.render(&params).unwrap();

    let response = Response::builder(200)
        .body(body)
        .content_type(mime::HTML)
        .build();
    Ok(response)
}

async fn serve_static(req: &Request<State>) -> TideResult {
    let path: PathBuf = req.param("file_path")?.into();
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
        ));
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
    let slug = req.param("blog_slug")?;
    if let Some(cid) = cid_map.get(slug) {
        let entry = kore::db::get_entry_cid(pool, *cid).unwrap();

        let liquid = req.state().liquid_parser.as_ref();
        let template = std::fs::read_to_string("./templates/entry.html.liquid").unwrap();
        let template = liquid.parse(&template).unwrap();

        let params = liquid::object!({ "entry": entry });
        let body = template.render(&params).unwrap();

        let response = Response::builder(200)
            .body(body)
            .content_type(mime::HTML)
            .build();
        return Ok(response);
    }
    debug!("Tried to serve blog/{} but it does not exist.", slug);
    Err(tide::http::Error::from_str(
        tide::StatusCode::NotFound,
        format!("Blog {} does not exist.", slug),
    ))
}
