use axum::{
    routing::{get, post, put, Route},
    Extension,
    Json,
    Router, response::sse::Event,
};
use serde_json::{json, Value};
use state::NodeStateReadHandle;
use vrrb_core::event_router::{DirectedEvent, Topic, Event as OtherEvent};

pub fn create_account_router(
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    state_read_handle: NodeStateReadHandle,
) -> Router {
    let event:DirectedEvent = (Topic::Network, OtherEvent::GetAccount("".to_string()));
    Router::new()
        .route("/:id", get(get_account))
        .route("/:id", put(update_account))
        .route("/", post(create_account(event)))
        .layer(Extension(String::from("account route")))
        .layer(Extension(events_tx))
        .layer(Extension(state_read_handle))
}

// NOTE: example, do not copy this verbatim, read currently open PRs to understand whats in
// progress
async fn get_account(
    Extension(events_tx): Extension<tokio::sync::mpsc::UnboundedSender<DirectedEvent>>,
    Extension(state_read_handle): Extension<NodeStateReadHandle>,
) -> Json<Value> {
    // Read some data
    let state_values = state_read_handle.values();

    // then publish a write with some data if needed
    // NOTE: dont unwrap in production code
    // events_tx
    //     .send(Event::SomeEvent(state_values.clone()))
    //     .await
    //     .unwrap();

    Json(json!({
        "account": "dummy_account_status",
        "state_values": state_values,
    }))
}

async fn create_account() {
    todo!()
}

async fn update_account() {
    todo!()
}

async fn create_key_router() {
    todo!()
}

async fn create_token_router() {
    todo!()
}

async fn get_key() {
    todo!()
}

async fn create_token() {
    todo!()
}

async fn update_token() {
    todo!()
}

async fn get_token() {
    todo!()
}

async fn delete_token() {
    todo!()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::{Service, ServiceExt};

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
