use axum::{
    routing::{get, post, put, Route},
    Extension, Json, Router,
};
use serde_json::{json, Value};

pub fn create_account_router() -> Router {
    Router::new()
        .route("/:id", get(get_account))
        .route("/:id", put(update_account))
        .route("/", post(create_account))
        .layer(Extension(String::from("account route")))
}

async fn get_account(Extension(state): Extension<String>) -> Json<Value> {
    Json(json!({
        "account": "dummy_account_status",
        "state": state,
    }))
}

async fn create_account() {
    todo!()
}

async fn update_account() {
    todo!()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::Service;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn get_account_returns_available_accounts() {
        let mut router = create_account_router();

        let request = Request::builder()
            .uri("/:id")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = router.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
