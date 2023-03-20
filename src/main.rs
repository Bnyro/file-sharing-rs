pub mod server;
pub mod template;
pub mod utils;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use server::serve_static;
use std::{
    collections::HashMap,
    fs::{create_dir_all, read_dir},
    net::SocketAddr,
};
use template::parse_template;
use tokio::{fs::File, io::AsyncWriteExt};
use utils::{generate_hash, humanize_bytes};

static FILESDIR: &str = "files";
static STATICDIR: &str = "static";
static SIZELIMIT: usize = 100;
static DELIMITER: &str = "_";
static DATE_FORMAT: &str = "%d.%m.%Y, %H:%M";

#[tokio::main]
async fn main() {
    let _ = create_dir_all(FILESDIR);

    let app = Router::new()
        .route("/", get(root))
        .route("/static/:name", get(get_static))
        .route("/all", get(get_files))
        .route("/upload", post(upload))
        .route("/:hash", get(get_upload))
        .route("/files/:name", get(get_file))
        .layer(DefaultBodyLimit::max(SIZELIMIT * 1024 * 1024));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> Html<String> {
    Html(parse_template(
        include_str!("../templates/home.html"),
        "File sharing with ease",
        HashMap::new(),
    ))
}

async fn get_files() -> Html<String> {
    let files = read_dir(FILESDIR).expect("Unable to read directory!");
    let mut html: String = String::from("");

    files.filter_map(|f| f.ok()).for_each(|file| {
        let name = file.file_name();
        let lossy_name = name.to_string_lossy();
        let file_path = lossy_name.split_once(DELIMITER).unwrap();
        let metadata = file.metadata().unwrap();
        let file_size = humanize_bytes(metadata.len() as f64);
        html += format!(
            "<li><a href=\"{}\">{} ({})</a></li>",
            file_path.0, file_path.1, file_size
        )
        .as_str();
    });

    Html(parse_template(&html, "All files", HashMap::new()))
}

async fn upload(mut multipart: Multipart) -> Redirect {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        let hash = generate_hash(16);
        let hashed_name = format!("{}/{}{}{}", FILESDIR, hash, DELIMITER, file_name);

        let mut file = File::create(hashed_name)
            .await
            .expect("No permissions to create files!");
        let _ = file.write_all(&data).await;
        return Redirect::permanent(&hash);
    }
    return Redirect::permanent("/");
}

async fn get_upload(Path(hash): Path<String>) -> Html<String> {
    let files = read_dir(FILESDIR).expect("Unable to read directory!");

    for f in files {
        let file = skip_fail!(f);
        let name = file.file_name();
        let curr_file_name = name.to_str().unwrap();
        let file_name_parts = curr_file_name.split_once(DELIMITER).unwrap();
        if file_name_parts.0 != hash {
            continue;
        };

        let file_name = curr_file_name.to_string();

        let mut arguments: HashMap<&str, &str> = HashMap::new();
        arguments.insert("path", file_name.as_str());
        arguments.insert("name", file_name.split_once(DELIMITER).unwrap().1);
        let metadata = file.metadata().unwrap();
        let file_size = humanize_bytes(metadata.len() as f64);
        arguments.insert("size", &file_size);
        let date = metadata.created().unwrap();
        let datetime: DateTime<Utc> = date.into();
        let formatted = datetime.format(DATE_FORMAT).to_string();
        arguments.insert("date", &formatted);

        return Html(parse_template(
            include_str!("../templates/file.html"),
            arguments.get("name").unwrap(),
            arguments,
        ));
    }

    return Html(parse_template(
        include_str!("../templates/notfound.html"),
        "Not found",
        HashMap::new(),
    ));
}

async fn get_file(Path(name): Path<String>) -> impl IntoResponse {
    let file_path = format!("{}/{}", FILESDIR, name);
    serve_static(file_path).await
}

async fn get_static(Path(name): Path<String>) -> impl IntoResponse {
    let file_path = format!("{}/{}", STATICDIR, name);
    serve_static(file_path).await
}
