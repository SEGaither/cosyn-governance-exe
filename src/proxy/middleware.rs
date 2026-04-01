use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    log::info!("{} {}", method, uri);
    let response = next.run(request).await;
    log::info!("{} {} -> {}", method, uri, response.status());
    response
}
