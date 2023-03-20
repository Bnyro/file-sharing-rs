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
use include_dir::{include_dir, Dir};
use server::{serve_static, static_response};
use std::{
    collections::HashMap,
    env,
    fs::{create_dir_all, read_dir},
    net::SocketAddr,
};
use template::{get_template, parse_template};
use tokio::{
    fs::{remove_file, File},
    io::AsyncWriteExt,
};
use utils::{generate_hash, humanize_bytes};

static FILESDIR: &str = "files";
static STATIC: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");
static SIZELIMIT: usize = 100;
static DELIMITER: &str = "_";
static DATE_FORMAT: &str = "%d.%m.%Y, %H:%M";

#[tokio::main]
async fn main() {
    let _ = create_dir_all(FILESDIR);

    let port = env::var("PORT")
        .unwrap_or(String::from("3000"))
        .parse()
        .expect("Invalid port!");

    let app = Router::new()
        .route("/", get(root))
        .route("/static/:name", get(get_static))
        .route("/all", get(get_files))
        .route("/upload", post(upload))
        .route("/:hash", get(get_upload))
        .route("/files/:name", get(get_file))
        .route("/delete/:name", get(delete_file))
        .layer(DefaultBodyLimit::max(SIZELIMIT * 1024 * 1024));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> Html<String> {
    Html(parse_template(
        get_template("home.html"),
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

async fn upload(mut multipart: Multipart) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        let file_size = humanize_bytes(data.len() as f64);

        let hash = generate_hash(16);
        let hashed_name = format!("{}{}{}", hash, DELIMITER, file_name);

        let mut file = File::create(format!("{}/{}", FILESDIR, hashed_name))
            .await
            .expect("No permissions to create files!");
        let _ = file.write_all(&data).await;

        let mut arguments: HashMap<&str, &str> = HashMap::new();
        arguments.insert("name", file_name.as_str());
        arguments.insert("path", hashed_name.as_str());
        arguments.insert("hash", hash.as_str());
        arguments.insert("size", file_size.as_str());

        return Html(parse_template(
            get_template("uploaded.html"),
            "Upload done",
            arguments,
        ))
        .into_response();
    }
    return Redirect::permanent("./").into_response();
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
            get_template("file.html"),
            arguments.get("name").unwrap(),
            arguments,
        ));
    }

    return Html(parse_template(
        get_template("notfound.html"),
        "Not found",
        HashMap::new(),
    ));
}

async fn delete_file(Path(name): Path<String>) -> Redirect {
    let _ = remove_file(format!("{}/{}", FILESDIR, name)).await;
    return Redirect::permanent("./");
}

async fn get_file(Path(name): Path<String>) -> impl IntoResponse {
    let file_path = format!("{}/{}", FILESDIR, name);
    serve_static(file_path).await
}

async fn get_static(Path(name): Path<String>) -> impl IntoResponse {
    let file = STATIC.get_file(&name);
    let mime_type = mime_guess::from_path(&name).first_or_text_plain();

    return if let Some(file) = file {
        static_response(mime_type, Some(file.contents().to_vec()))
    } else {
        static_response(mime_type, None)
    }
    .await;
}
