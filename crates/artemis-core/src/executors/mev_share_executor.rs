use crate::types::Executor;
use alloy::providers::ext::sign_flashbots_payload;
use alloy::rpc::types::mev::MevSendBundle;
use alloy::signers::Signer;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{Client, Url, header};
use serde::Serialize;
use tracing::{error, info};

/// An executor that sends bundles to the MEV-share matchmaker.
pub struct MevshareExecutor<S> {
    client: Client,
    relay_url: Url,
    auth_signer: S,
}

impl<S> MevshareExecutor<S>
where
    S: Signer + Send + Sync + 'static,
{
    pub fn new(auth_signer: S) -> Self {
        let client = Client::builder()
            .user_agent("artemis-mevshare-executor")
            .build()
            .expect("failed to build MEV-share HTTP client");

        let relay_url =
            Url::parse("https://relay.flashbots.net").expect("invalid MEV-share relay url");

        Self {
            client,
            relay_url,
            auth_signer,
        }
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: &'static str,
    method: &'static str,
    params: [T; 1],
    id: u64,
}

#[async_trait]
impl<S> Executor<MevSendBundle> for MevshareExecutor<S>
where
    S: Signer + Send + Sync + 'static,
{
    async fn execute(&self, bundle: MevSendBundle) -> Result<()> {
        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "mev_sendBundle",
            params: [bundle],
            id: 1,
        };

        let body =
            serde_json::to_string(&rpc_request).context("failed to serialize MEV-share bundle")?;
        let signature = sign_flashbots_payload(body.clone(), &self.auth_signer).await?;

        let response = self
            .client
            .post(self.relay_url.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header("X-Flashbots-Signature", signature)
            .body(body.clone())
            .send()
            .await
            .context("failed to send MEV-share bundle request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            error!(
                "MEV-share relay returned error status {} with body {}",
                status, text
            );
            return Ok(());
        }

        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) if value.get("error").is_some() => {
                error!("MEV-share relay error response: {}", value);
            }
            Ok(value) => {
                info!("MEV-share relay response: {}", value);
            }
            Err(_) => {
                info!("MEV-share relay response: {}", text);
            }
        }

        Ok(())
    }
}
