use actix_web::{get, web, App, HttpResponse, HttpServer, ResponseError};
use actix_web::{http::header, post};
use thiserror::Error;
use askama::Template;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params; 
use serde::Deserialize;

struct TodoEntry {
    id: u32,
    text: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    entries: Vec<TodoEntry>,
}

#[derive(Deserialize)]
struct AddParams {
    text: String,
}

#[derive(Deserialize)]
struct DeletePalams {
    id: u32,
}
// thiserrorを使って独自の構造体にエラー処理を定義
#[derive(Error, Debug)]
enum MyError {
    #[error("Failed to render HTML")]
    AskamaError(#[from] askama::Error),
    // askama::Error型をMyError型に変換するFromトレイトを自動で実装する

    #[error("Failed to get connection")]
    ConnectionPoolError(#[from] r2d2::Error),

    #[error("Failed to execute SQL statement")]
    SQLiteError(#[from] rusqlite::Error),
}

impl ResponseError for MyError {}

#[get("/")]
async fn index(db: web::Data<Pool<SqliteConnectionManager>>) -> Result<HttpResponse, MyError> {
    
    let connection = db.get()?;
    let mut statement = connection.prepare("SELECT id, text FROM todo")?;

    let rows = statement.query_map(params![], |row| {
        let id = row.get(0)?;
        let text = row.get(1)?;
        Ok( TodoEntry{ id, text } )
    })?;
    
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    
    let html = IndexTemplate { entries };
    let response_body = html.render()?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(response_body))
}

#[post("/add")]
async fn add_todo(
    params: web::Form<AddParams>,
    db: web::Data<r2d2::Pool<SqliteConnectionManager>>
) -> Result<HttpResponse, MyError> {
    let connection = db.get()?;
    connection.execute("INSERT INTO todo (text) VALUES (?)", &[&params.text])?;
    Ok( HttpResponse::SeeOther()
        .header(header::LOCATION, "/")
        .finish()
    )
}

#[post("/delete")]
async fn delete_todo(
    params: web::Form<DeletePalams>,
    db: web::Data<r2d2::Pool<SqliteConnectionManager>>
) -> Result<HttpResponse, MyError> {
    let connection = db.get()?;
    connection.execute("DELETE FROM todo WHERE id=?", &[&params.id])?;
    Ok(HttpResponse::SeeOther()
        .header(header::LOCATION, "/")
        .finish()
    )
}


#[actix_rt::main]
async fn main() -> Result<(), actix_web::Error> {
    
    let manager = SqliteConnectionManager::file("todo.db");
    let pool = Pool::new(manager).expect("failed to initialize the connection pool");

    let connection = pool
        .get()
        .expect("Failed to get the connection from the pool");

    connection.execute(
        "CREATE TABLE IF NOT EXISTS todo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL
        )",
        params![],
    )
    .expect("failed to create a table `todo`.");

    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(add_todo)
            .service(delete_todo)
            .data(pool.clone())
    })  
    .bind("0.0.0.0:8080")?
    .run()        
    .await?;

    Ok(())
}