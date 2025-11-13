use crate::types::{Collector, CollectorStream};
use alloy::primitives::{B256, U64};
use alloy::providers::Provider;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A collector that listens for new blocks, and generates a stream of
/// [events](NewBlock) which contain the block number and hash.
pub struct BlockCollector<M> {
    provider: Arc<M>,
}

/// A new block event, containing the block number and hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBlock {
    pub hash: B256,
    pub number: U64,
}

impl<M> BlockCollector<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self { provider }
    }
}

/// Implementation of the [Collector](Collector) trait for the [BlockCollector](BlockCollector).
/// This implementation uses the [PubsubClient](PubsubClient) to subscribe to new blocks.
#[async_trait]
impl<M> Collector<NewBlock> for BlockCollector<M>
where
    M: Provider + Send + Sync + 'static,
{
    async fn get_event_stream<'life1>(&self) -> Result<CollectorStream<'life1, NewBlock>> {
        let stream = self
            .provider
            .subscribe_blocks()
            .await?
            .into_stream()
            .map(|header| NewBlock {
                hash: header.hash,
                number: U64::from(header.number),
            });
        Ok(Box::pin(stream))
    }
}
