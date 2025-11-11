use crate::types::{Collector, CollectorStream};
use alloy::providers::Provider;
use alloy::rpc::types::eth::{Filter, Log};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// A collector that listens for new blockchain event logs based on a [Filter](Filter),
/// and generates a stream of [events](Log).
pub struct LogCollector<M> {
    provider: Arc<M>,
    filter: Filter,
}

impl<M> LogCollector<M> {
    pub fn new(provider: Arc<M>, filter: Filter) -> Self {
        Self { provider, filter }
    }
}

/// Implementation of the [Collector](Collector) trait for the [LogCollector](LogCollector).
/// This implementation uses the [PubsubClient](PubsubClient) to subscribe to new logs.
#[async_trait]
impl<M> Collector<Log> for LogCollector<M>
where
    M: Provider + Send + Sync + 'static,
{
    async fn get_event_stream<'life1>(&self) -> Result<CollectorStream<'life1, Log>> {
        let stream = self.provider.subscribe_logs(&self.filter).await?;
        Ok(Box::pin(stream.into_stream()))
    }
}
