use crate::types::{Collector, CollectorStream};
use alloy::providers::Provider;
use alloy::rpc::types::eth::Transaction;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// A collector that listens for new transactions in the mempool, and generates a stream of
/// [events](Transaction) which contain the transaction.
pub struct MempoolCollector<M> {
    provider: Arc<M>,
}

impl<M> MempoolCollector<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self { provider }
    }
}

/// Implementation of the [Collector](Collector) trait for the [MempoolCollector](MempoolCollector).
/// This implementation subscribes to pending transactions via Alloy's pubsub support.
#[async_trait]
impl<M> Collector<Transaction> for MempoolCollector<M>
where
    M: Provider + Send + Sync + 'static,
{
    async fn get_event_stream<'life1>(&self) -> Result<CollectorStream<'life1, Transaction>> {
        let stream = self
            .provider
            .subscribe_full_pending_transactions()
            .await?
            .into_stream();
        Ok(Box::pin(stream))
    }
}
