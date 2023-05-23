use reqwest::{header, Client, Method, RequestBuilder, Response, Url};

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct HttpClientBuilder {
    base_url: Url,
    client: Client,
    headers: reqwest::header::HeaderMap,
}

impl HttpClientBuilder {
    pub fn new(_base_url: String) -> Result<Self> {
        let base_url = match Url::parse(&_base_url) {
            Ok(base_url) => base_url,
            Err(e) => return Err(Error::UrlError(e)),
        };

        let client = Client::new();
        let headers = reqwest::header::HeaderMap::new();
        Ok(Self {
            base_url,
            client,
            headers,
        })
    }

    pub fn default_headers(mut self) -> Self {
        self.headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        self
    }

    pub fn build(self) -> HttpClient {
        HttpClient {
            base_url: self.base_url,
            client: self.client,
            headers: self.headers,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpClient {
    base_url: Url,
    client: Client,
    headers: reqwest::header::HeaderMap,
}

impl HttpClient {
    pub async fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = self.base_url.join(path).unwrap();
        self.client
            .request(method, url)
            .headers(self.headers.clone())
    }

    pub async fn get(&self, path: &str) -> Result<Response> {
        self.request(Method::GET, path)
            .await
            .send()
            .await
            .map_err(Error::RequestError)
    }

    pub async fn post(&self, path: &str, body: &str) -> Result<Response> {
        self.request(Method::POST, path)
            .await
            .body(body.to_owned())
            .send()
            .await
            .map_err(Error::RequestError)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_get_request() {
        let mock_server = MockServer::start().await;
        let mock_response =
            ResponseTemplate::new(200).set_body_json(json!({"message": "Hello, World!"}));
        Mock::given(path("/test"))
            .respond_with(mock_response)
            .mount(&mock_server)
            .await;

        let url = format!("{}{}", "http://", mock_server.address().to_string());
        let http_client = HttpClientBuilder::new(url)
            .unwrap()
            .default_headers()
            .build();

        let response = http_client.get("/test").await.unwrap();
        assert_eq!(response.status(), 200);
        let body_str = response.text().await.unwrap();
        let body_json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(body_json, json!({"message": "Hello, World!"}));
    }

    #[tokio::test]
    async fn test_post_request() {
        let mock_server = MockServer::start().await;
        let mock_response = ResponseTemplate::new(200).set_body_json(json!({"success": true}));
        Mock::given(path("/test"))
            .respond_with(mock_response)
            .mount(&mock_server)
            .await;

        let url = format!("{}{}", "http://", mock_server.address().to_string());
        let http_client = HttpClientBuilder::new(url)
            .unwrap()
            .default_headers()
            .build();

        let response = http_client
            .post("/test", "{\"message\":\"Hello, World!\"}")
            .await
            .unwrap();
        let body_str = response.text().await.unwrap();
        let body_json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(body_json, json!({"success": true}));
    }
}
