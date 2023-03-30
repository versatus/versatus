use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{anyhow, Result};
use http::{HttpClient, HttpClientBuilder};
use mempool::TxnRecord;
use reqwest::StatusCode;
use serde_json;

use crate::http;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexerClientConfig {
    pub base_url: SocketAddr,
}

impl Default for IndexerClientConfig {
    fn default() -> Self {
        let base_url = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3444);

        Self { base_url }
    }
}

#[derive(Debug, Clone)]
pub struct IndexerClient {
    client: HttpClient,
}

impl IndexerClient {
    pub fn new(config: IndexerClientConfig) -> Result<Self, reqwest::Error> {
        let client = HttpClientBuilder::new(&config.base_url)?
            .default_headers()
            .build();
        // .map_err(|e| Error::from(e))?;

        Ok(Self { client })
    }

    pub async fn post_tx(mut self, txn_record: &TxnRecord) -> Result<StatusCode, reqwest::Error> {
        let req_json = serde_json::to_string(txn_record)
            .map_err(|e| anyhow!("Failed to serialize txn_record to json: {}", e));

        let response = self
            .client
            .post("/transactions", &req_json.unwrap())
            .await
            .map_err(|e| anyhow!("Failed to serialize txn_record to json: {}", e));

        Ok(response.unwrap().status())
    }
}

#[cfg(test)]
mod tests {
    use mempool::{TxnRecord, TxnStatus};
    use serde_json::json;
    use vrrb_core::txn::{TransactionDigest, Txn};
    use wiremock::{
        http::Method,
        matchers::{method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use super::*;

    #[tokio::test]
    async fn test_post_tx_success() {
        let mock_server = MockServer::start().await;

        let base_url = mock_server.address();

        let config = IndexerClientConfig {
            base_url: *base_url,
        };
        let client = IndexerClient::new(config).unwrap();

        let txn = Txn::default();
        let txn_record = TxnRecord::new(txn);
        let expected_body = json!(txn_record).to_string();

        let response = ResponseTemplate::new(200).set_body_json(txn_record.to_owned());

        Mock::given(method("POST"))
            .and(path("/transactions"))
            .respond_with(response)
            .mount(&mock_server)
            .await;

        let result = client.post_tx(&txn_record).await;

        assert!(result.is_ok());
        let request_body = mock_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap()
            .body
            .to_vec();
        // assert_eq!(request_body, expected_body.as_bytes());
    }

    #[tokio::test]
    async fn test_post_tx_failure() {
        let mock_server = wiremock::MockServer::start().await;

        let mock = wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/transactions"))
            .respond_with(wiremock::ResponseTemplate::new(500));

        wiremock::Mock::mount(mock, &mock_server).await;

        let base_url = mock_server.address();

        let indexer_config = IndexerClientConfig {
            base_url: *base_url,
        };
        let indexer_client = IndexerClient::new(indexer_config).unwrap();

        let txn = Txn::default();
        let txn_record = TxnRecord::new(txn);
        let expected_body = json!(txn_record).to_string();

        let result = indexer_client.post_tx(&txn_record).await;

        // assert!(result.is_err());

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::Post);
        assert_eq!(requests[0].url.path(), "/transactions");
    }
}
