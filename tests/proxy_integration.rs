use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let resp = app
        .oneshot(Request::builder().uri("/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_message_returns_governance_breach() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": ""}]
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Breach behavior: Annotate — returns 200 with breach info in response
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let content = resp_json["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(content.contains("GOVERNANCE BREACH"), "Expected breach annotation in response");
}

#[tokio::test]
async fn streaming_request_rejected() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn breach_emits_telemetry() {
    cosyn::telemetry::take_log(); // clear
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "messages": [{"role": "user", "content": ""}]
    });
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let log = cosyn::telemetry::take_log();
    let joined = log.join("\n");
    assert!(joined.contains("input_received"), "missing input_received event");
    assert!(joined.contains("final_release_decision"), "missing final_release_decision event");
}
