use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::http::{
    routes::{accounts, health},
    HttpApiRouterConfig,
};

pub fn create_router(config: &HttpApiRouterConfig) -> Router {
    Router::new()
        .route("/", get(|| async { "index" }))
        .route("/health", get(health::health_check))
        .nest("/accounts", accounts::create_account_router())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

#[cfg(test)]
mod tests {
    use crate::http::HttpApiRouterConfigBuilder;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::net::SocketAddr;
    use tower::{Service, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn index_should_exist() {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let config = HttpApiRouterConfigBuilder::default()
            .address(listener.local_addr().unwrap())
            .api_title("Node HTTP API")
            .api_version("1.0")
            .server_timeout(None)
            .build();

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
