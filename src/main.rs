use axum::{
    body::{self, Empty, Full},
    extract::{Multipart, Path},
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use rand::Rng;
use std::{
    fs::{create_dir_all, read_dir, File},
    io::{Read, Write},
    iter,
    net::SocketAddr,
    path::PathBuf,
};

static FILESDIR: &str = "files";
static SIZELIMIT: u32 = 100;
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

#[tokio::main]
async fn main() {
    let _ = create_dir_all(FILESDIR);

    let app = Router::new()
        .route("/", get(root))
        .route("/all", get(get_files))
        .route("/upload", post(upload))
        .route("/:hash", get(get_upload))
        .route("/files/:name", get(get_file));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> Html<&'static str> {
    Html(include_str!("../templates/home.html"))
}

async fn get_files() -> Html<String> {
    let files = read_dir(FILESDIR).expect("Unable to read directory!");
    let mut html: String = String::from("");

    files.for_each(|file| {
        html += file
            .unwrap()
            .file_name()
            .to_string_lossy()
            .to_string()
            .as_str();
        html += "<br />";
    });

    Html(html)
}

async fn upload(mut multipart: Multipart) -> Redirect {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if data.len() > (SIZELIMIT * 1024).try_into().unwrap() {
            return Redirect::temporary("/");
        }

        let hash = generate_hash(10);
        let hashed_name = format!("{}/{}_{}", FILESDIR, hash, file_name);

        let mut file = File::create(hashed_name).expect("No permissions to create files!");
        let _ = file.write_all(&data);
        return Redirect::permanent(&hash);
    }
    return Redirect::permanent("/");
}

async fn get_upload(Path(hash): Path<String>) -> Html<String> {
    let files = read_dir(FILESDIR).expect("Unable to read directory!");

    let mut file_name: Option<String> = None;
    let mut file: Option<PathBuf> = None;

    for f in files {
        if let Ok(f) = f {
            let name = f.file_name();
            let curr_file_name = name.to_str().unwrap();
            let file_name_parts = curr_file_name.split_once("_").unwrap();
            if file_name_parts.0 == hash {
                file_name = Some(curr_file_name.to_string());
                file = Some(f.path());
                break;
            }
        }
    }

    if file.is_none() {
        return Html(include_str!("../templates/notfound.html").to_string());
    }

    Html(
        include_str!("../templates/file.html")
            .replace("{{path}}", file_name.clone().unwrap().as_str())
            .replace("{{name}}", file_name.unwrap().split_once("_").unwrap().1)
            .to_string(),
    )
}

async fn get_file(Path(name): Path<String>) -> impl IntoResponse {
    let mime_type = mime_guess::from_path(name.clone()).first_or_text_plain();

    let file_path = format!("{}/{}", FILESDIR, name);

    return if let Ok(mut file) = File::open(file_path) {
        let mut response_bytes: Vec<u8> = vec![];
        file.read_to_end(&mut response_bytes)
            .expect("Can't read file!");
        Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(response_bytes)))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap()
    };
}

fn generate_hash(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(len).collect()
}
