use std::sync::Arc;

use alloy::{
    eips::Encodable2718,
    network::{Ethereum, NetworkWallet, TransactionBuilder},
    primitives::hex,
    providers::{ext::sign_flashbots_payload, Provider},
    rpc::types::eth::TransactionRequest,
    signers::Signer,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{header, Client, Url};
use serde::Serialize;
use tracing::{error, info};

use crate::types::Executor;

/// A bundle of transactions to send to the Flashbots relay.
pub type FlashbotsBundle = Vec<TransactionRequest>;

/// A Flashbots executor that sends transactions to the Flashbots relay using Alloy primitives.
pub struct FlashbotsExecutor<P, TxSigner, AuthSigner> {
    /// Provider used for fetching network data such as block numbers.
    provider: Arc<P>,
    /// HTTP client pointing at the Flashbots relay.
    client: Client,
    /// Flashbots relay endpoint.
    relay_url: Url,
    /// Signer used to sign transactions before broadcasting.
    tx_signer: TxSigner,
    /// Signer used to authenticate requests with `X-Flashbots-Signature`.
    auth_signer: AuthSigner,
}

impl<P, TxSigner, AuthSigner> FlashbotsExecutor<P, TxSigner, AuthSigner>
where
    P: Provider + Send + Sync + 'static,
    TxSigner: Signer + NetworkWallet<Ethereum> + Send + Sync + 'static,
    AuthSigner: Signer + Send + Sync + 'static,
{
    pub fn new(provider: Arc<P>, tx_signer: TxSigner, auth_signer: AuthSigner, relay_url: Url) -> Self {
        let client = Client::builder()
            .user_agent("artemis-flashbots-executor")
            .build()
            .expect("failed to build Flashbots HTTP client");

        Self {
            provider,
            client,
            relay_url,
            tx_signer,
            auth_signer,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct FlashbotsBundlePayload {
    txs: Vec<String>,
    #[serde(rename = "blockNumber")]
    block_number: String,
    #[serde(rename = "minTimestamp", skip_serializing_if = "Option::is_none")]
    min_timestamp: Option<u64>,
    #[serde(rename = "maxTimestamp", skip_serializing_if = "Option::is_none")]
    max_timestamp: Option<u64>,
    #[serde(rename = "revertingTxHashes", skip_serializing_if = "Option::is_none")]
    reverting_tx_hashes: Option<Vec<String>>,
    #[serde(rename = "replacementUuid", skip_serializing_if = "Option::is_none")]
    replacement_uuid: Option<String>,
    #[serde(rename = "stateBlockNumber", skip_serializing_if = "Option::is_none")]
    state_block_number: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: &'static str,
    method: &'static str,
    params: [T; 1],
    id: u64,
}

#[async_trait]
impl<P, TxSigner, AuthSigner> Executor<FlashbotsBundle> for FlashbotsExecutor<P, TxSigner, AuthSigner>
where
    P: Provider + Send + Sync + 'static,
    TxSigner: Signer + NetworkWallet<Ethereum> + Send + Sync + 'static,
    AuthSigner: Signer + Send + Sync + 'static,
{
    /// Send a bundle of transactions to the Flashbots relay.
    async fn execute(&self, bundle: FlashbotsBundle) -> Result<()> {
        if bundle.is_empty() {
            return Ok(());
        }

        let mut raw_txs = Vec::with_capacity(bundle.len());

        for mut tx in bundle {
            if tx.from.is_none() {
                tx.from = Some(self.tx_signer.address());
            }
            let envelope = tx
                .clone()
                .build(&self.tx_signer)
                .await
                .context("failed to sign flashbots transaction")?;
            let raw = envelope.encoded_2718();
            raw_txs.push(hex::encode_prefixed(raw));
        }

        let current_number = self
            .provider
            .get_block_number()
            .await
            .context("failed to fetch latest block number for flashbots bundle")?;
        let target_block = current_number + 1;

        let payload = FlashbotsBundlePayload {
            txs: raw_txs,
            block_number: format!("0x{:x}", target_block),
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: None,
            replacement_uuid: None,
            state_block_number: None,
        };

        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "eth_sendBundle",
            params: [payload],
            id: 1,
        };

        let body = serde_json::to_string(&rpc_request).context("failed to serialize flashbots bundle")?;
        let signature = sign_flashbots_payload(body.clone(), &self.auth_signer)
            .await
            .context("failed to sign flashbots payload")?;

        let response = self
            .client
            .post(self.relay_url.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header("X-Flashbots-Signature", signature)
            .body(body.clone())
            .send()
            .await
            .context("failed to send flashbots bundle request")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            error!(
                "Flashbots relay returned error status {} with body {}",
                status, text
            );
        } else {
            info!("Flashbots relay response: {}", text);
        }

        Ok(())
    }
}
