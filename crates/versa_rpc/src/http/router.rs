use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::http::{
    routes::{accounts, health},
    HttpApiRouterConfig,
};

pub fn create_router(_config: &HttpApiRouterConfig) -> Router {
    Router::new()
        .route("/", get(|| async { "index" }))
        .route("/health", get(health::health_check))
        .nest("/accounts", accounts::create_account_router())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::{Service, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn index_should_exist() {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let address = listener.local_addr().unwrap();

        let api_title = "Node HTTP API".to_string();
        let api_version = "1.0".to_string();

        let config = HttpApiRouterConfig {
            address,
            api_title,
            api_version,
            server_timeout: None,
        };

        let mut router = create_router(&config);

        let request = Request::builder()
            .uri("/")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = router.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
