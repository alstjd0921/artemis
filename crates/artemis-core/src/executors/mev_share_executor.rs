use crate::types::Executor;
use alloy::providers::{Provider, ext::MevApi};
use alloy::rpc::types::mev::MevSendBundle;
use alloy::signers::Signer;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info};

/// An executor that sends bundles to the MEV-share matchmaker.
pub struct MevshareExecutor<P, S> {
    mev_provider: Arc<P>,
    auth_signer: S,
}

impl<P, S> MevshareExecutor<P, S>
where
    P: Provider + Send + Sync + 'static,
    S: Signer + Clone + Send + Sync + 'static,
{
    pub fn new(mev_provider: Arc<P>, auth_signer: S) -> Self {
        Self {
            mev_provider,
            auth_signer,
        }
    }
}

#[async_trait]
impl<P, S> Executor<MevSendBundle> for MevshareExecutor<P, S>
where
    P: Provider + Send + Sync + 'static,
    S: Signer + Clone + Send + Sync + 'static,
{
    async fn execute(&self, bundle: MevSendBundle) -> Result<()> {
        info!("MEV-share relay bundle: {:?}", bundle);
        match self
            .mev_provider
            .send_mev_bundle(bundle.clone())
            .with_auth(self.auth_signer.clone())
            .await
        {
            Ok(Some(response)) => {
                info!("MEV-share relay bundle response: {}", response.bundle_hash);
            }
            Ok(None) => {
                info!("MEV-share no bundle response");
            }
            Err(e) => {
                error!("failed to send mev bundle: {}", e);
            }
        }

        Ok(())
    }
}
