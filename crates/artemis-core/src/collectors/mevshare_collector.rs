use alloy::rpc::types::mev::mevshare::Event;
use crate::types::{Collector, CollectorStream, Events, MEV_SHARE};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{trace, warn};

/// A collector that streams from MEV-Share SSE endpoint
/// and generates [events](Event), which return tx hash, logs, and bundled txs.
pub struct MevShareCollector;

impl MevShareCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Implementation of the [Collector](Collector) trait for the
/// [MevShareCollector](MevShareCollector).
#[async_trait]
impl Collector<Event> for MevShareCollector {
    async fn get_event_stream<'life1>(&self) -> Result<CollectorStream<'life1, Event>> {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                let client = mev_share_sse::EventClient::default().with_max_retries(u64::MAX);
                let mut stream = client.events(MEV_SHARE).await.unwrap();

                while let Some(event) = stream.next().await {
                    match event {
                        Ok(event) => {
                            if tx.send(event).is_err() {
                                trace!("all MEV-share receivers dropped, stopping stream");
                                return;
                            }
                        }
                        Err(err) => {
                            warn!("MEV-share SSE stream error: {err}");
                            break;
                        }
                    };
                }
                if tx.is_closed() {
                    trace!("MEV-share event receiver dropped, stopping collector loop");
                    break;
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mevshare_collector() {
        let collector = MevShareCollector::new();
        let mut stream = collector
            .get_event_stream()
            .await
            .expect("failed to get event");

        while let Some(event) = stream.next().await {
            dbg!(&event);
            break;
        }
    }
}
