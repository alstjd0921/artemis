use crate::types::Executor;
use alloy::providers::Provider;
use alloy::providers::ext::MevApi;
use alloy::rpc::types::mev::EthSendPrivateTransaction;
use alloy::signers::Signer;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info};

pub struct FlashbotsSingleExecutor<P, AuthSigner> {
    mev_provider: Arc<P>,
    auth_signer: AuthSigner,
}

impl<P, AuthSigner> FlashbotsSingleExecutor<P, AuthSigner>
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
impl<P, AuthSigner> Executor<EthSendPrivateTransaction> for FlashbotsSingleExecutor<P, AuthSigner>
where
    P: Provider + Send + Sync + 'static,
    AuthSigner: Signer + Clone + Send + Sync + 'static,
{
    async fn execute(&self, tx: EthSendPrivateTransaction) -> anyhow::Result<()> {
        match self
            .mev_provider
            .send_private_transaction(tx)
            .with_auth(self.auth_signer.clone())
            .await
        {
            Ok(Some(response)) => {
                info!("Private tx sent successfully: {}", response);
            }
            Ok(None) => {
                info!("No private send response");
            }
            Err(err) => {
                error!(
                    "Failed to send private tx: {}",
                    err.to_string().replace("\n", "")
                )
            }
        }

        Ok(())
    }
}
