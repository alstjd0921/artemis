use std::sync::Arc;

use alloy::providers::ext::MevApi;
use alloy::rpc::types::mev::EthSendBundle;
use alloy::{providers::Provider, signers::Signer};
use anyhow::Result;
use async_trait::async_trait;
use tracing::{error, info};

use crate::types::Executor;

/// A Flashbots executor that sends transactions to the Flashbots relay using Alloy primitives.
pub struct FlashbotsExecutor<P, AuthSigner> {
    /// Flashbots relay provider.
    mev_provider: Arc<P>,
    /// Signer used to authenticate requests with `X-Flashbots-Signature`.
    auth_signer: AuthSigner,
}

impl<P, AuthSigner> FlashbotsExecutor<P, AuthSigner>
where
    P: Provider + Send + Sync + 'static,
    AuthSigner: Signer + Clone + Send + Sync + 'static,
{
    pub fn new(mev_provider: Arc<P>, auth_signer: AuthSigner) -> Self {
        Self {
            mev_provider,
            auth_signer,
        }
    }
}

#[async_trait]
impl<P, AuthSigner> Executor<EthSendBundle> for FlashbotsExecutor<P, AuthSigner>
where
    P: Provider + Send + Sync + 'static,
    AuthSigner: Signer + Clone + Send + Sync + 'static,
{
    /// Send a bundle of transactions to the Flashbots relay.
    async fn execute(&self, bundle: EthSendBundle) -> Result<()> {
        if bundle.txs.is_empty() {
            return Ok(());
        }

        match self
            .mev_provider
            .send_bundle(bundle)
            .with_auth(self.auth_signer.clone())
            .await
        {
            Ok(Some(response)) => {
                info!("Relay response: {}", response.bundle_hash);
            }
            Ok(None) => {
                info!("No relay response");
            }
            Err(e) => {
                error!("Failed to send bundle: {}", e);
            }
        }

        Ok(())
    }
}
