use axum::{
    body::{self, Empty, Full},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use mime_guess::Mime;
use tokio::{fs::File, io::AsyncReadExt};

pub async fn serve_static(path: String) -> impl IntoResponse {
    let mime_type = mime_guess::from_path(&path).first_or_text_plain();

    return if let Ok(mut file) = File::open(path).await {
        let mut response_bytes: Vec<u8> = vec![];
        file.read_to_end(&mut response_bytes)
            .await
            .expect("Can't read file!");
        static_response(mime_type, Some(response_bytes))
    } else {
        static_response(mime_type, None)
    }
    .await;
}

pub async fn static_response(
    mime_type: Mime,
    response_bytes: Option<Vec<u8>>,
) -> impl IntoResponse {
    return if let Some(content) = response_bytes {
        Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(content)))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap()
    };
}
