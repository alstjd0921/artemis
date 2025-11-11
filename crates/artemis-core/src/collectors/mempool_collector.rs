use crate::types::{Collector, CollectorStream};
use anyhow::Result;
use alloy::providers::Provider;
use alloy::rpc::types::eth::Transaction;
use async_trait::async_trait;
use futures::StreamExt;
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
    async fn get_event_stream(&self) -> Result<CollectorStream<'life0, Transaction>> {
        let provider = self.provider.clone();
        let stream = self.provider.subscribe_pending_transactions().await?;
        let stream = stream.into_stream().filter_map(move |hash| {
            let provider = provider.clone();
            async move {
                match provider.get_transaction_by_hash(hash).await {
                    Ok(Some(tx)) => Some(tx),
                    _ => None,
                }
            }
        });
        Ok(Box::pin(stream))
    }
}
