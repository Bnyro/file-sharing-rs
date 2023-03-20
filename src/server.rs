use axum::{
    body::{self, Empty, Full},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::{fs::File, io::AsyncReadExt};

pub async fn serve_static(path: String) -> impl IntoResponse {
    let mime_type = mime_guess::from_path(path.clone()).first_or_text_plain();

    return if let Ok(mut file) = File::open(path).await {
        let mut response_bytes: Vec<u8> = vec![];
        file.read_to_end(&mut response_bytes)
            .await
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
