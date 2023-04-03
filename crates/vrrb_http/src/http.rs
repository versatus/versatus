use std::net::SocketAddr;

use reqwest::{header, Client, Method, RequestBuilder, Response, Url};

#[derive(Debug, Clone)]
pub struct HttpClientBuilder {
    base_url: Url,
    client: Client,
    headers: reqwest::header::HeaderMap,
}

impl HttpClientBuilder {
    pub fn new(base_url: &SocketAddr) -> Result<Self, reqwest::Error> {
        let base_url = Url::parse(&format!("http://{}", base_url)).unwrap();

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
    // pub fn set_header(mut self, name: &str, value: &str) {
    //     self.headers.insert(name, value.parse().unwrap());
    // }

    pub async fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = self.base_url.join(path).unwrap();
        self.client
            .request(method, url)
            .headers(self.headers.clone())
    }

    pub async fn get(&self, path: &str) -> Result<Response, reqwest::Error> {
        self.request(Method::GET, path).await.send().await
    }

    pub async fn post(&self, path: &str, body: &str) -> Result<Response, reqwest::Error> {
        self.request(Method::POST, path)
            .await
            .body(body.to_owned())
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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

        // let base_url = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        // 3444);
        let http_client = HttpClientBuilder::new(mock_server.address())
            .unwrap()
            .default_headers()
            .build();

        let response = http_client.get("/test").await.unwrap();
        assert_eq!(response.status(), 200);
        // let body = response.text().await.unwrap();
        // assert_eq!(body, json!({"message": "Hello, World!"}));
    }

    #[tokio::test]
    async fn test_post_request() {
        let mock_server = MockServer::start().await;
        let mock_response = ResponseTemplate::new(200).set_body_json(json!({"success": true}));
        Mock::given(path("/test"))
            .respond_with(mock_response)
            .mount(&mock_server)
            .await;

        let http_client = HttpClientBuilder::new(mock_server.address())
            .unwrap()
            .default_headers()
            .build();

        let response = http_client
            .post("/test", "{\"message\":\"Hello, World!\"}")
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        // let body = response.json::<serde_json::Value>().await.unwrap();
        // assert_eq!(body, json!({"success": true}));
    }
}
