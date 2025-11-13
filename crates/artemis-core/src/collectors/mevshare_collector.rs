use crate::{
    mevshare::Event,
    types::{Collector, CollectorStream},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{trace, warn};

/// A collector that streams from MEV-Share SSE endpoint
/// and generates [events](Event), which return tx hash, logs, and bundled txs.
pub struct MevShareCollector {
    mevshare_sse_url: String,
}

impl MevShareCollector {
    pub fn new(mevshare_sse_url: String) -> Self {
        Self { mevshare_sse_url }
    }
}

/// Implementation of the [Collector](Collector) trait for the
/// [MevShareCollector](MevShareCollector).
#[async_trait]
impl Collector<Event> for MevShareCollector {
    async fn get_event_stream<'life1>(&self) -> Result<CollectorStream<'life1, Event>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let url = self.mevshare_sse_url.clone();
        let client = Client::new();

        tokio::spawn(async move {
            const INITIAL_BACKOFF_SECS: u64 = 1;
            const MAX_BACKOFF_SECS: u64 = 30;
            let mut backoff_delay = Duration::from_secs(INITIAL_BACKOFF_SECS);

            loop {
                if tx.is_closed() {
                    trace!("MEV-share event receiver dropped, stopping collector loop");
                    break;
                }

                let request = match client.get(&url).send().await {
                    Ok(resp) => resp,
                    Err(err) => {
                        warn!("failed to connect to MEV-share SSE endpoint: {err}");
                        sleep(backoff_delay).await;
                        backoff_delay =
                            (backoff_delay * 2).min(Duration::from_secs(MAX_BACKOFF_SECS));
                        continue;
                    }
                };

                let mut stream = request.bytes_stream();
                let mut buffer: Vec<u8> = Vec::new();
                backoff_delay = Duration::from_secs(INITIAL_BACKOFF_SECS);

                while let Some(chunk) = stream.next().await {
                    let chunk = match chunk {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            warn!("MEV-share SSE stream error: {err}");
                            break;
                        }
                    };

                    buffer.extend_from_slice(&chunk);

                    while let Some(event) = extract_event(&mut buffer) {
                        if event.is_empty() {
                            continue;
                        }

                        for line in event.lines() {
                            if let Some(data) = line.strip_prefix("data:") {
                                let payload = data.trim();
                                if payload.is_empty() || payload == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<Event>(payload) {
                                    Ok(evt) => {
                                        trace!("MEV-share event: {evt:?}");
                                        if tx.send(evt).is_err() {
                                            trace!(
                                                "all MEV-share receivers dropped, stopping stream"
                                            );
                                            return;
                                        }
                                    }
                                    Err(err) => {
                                        trace!("failed to parse MEV-share event: {err}");
                                    }
                                }
                            }
                        }
                    }
                }

                if tx.is_closed() {
                    trace!("MEV-share event receiver dropped, stopping collector loop");
                    break;
                }

                warn!(
                    "MEV-share SSE stream ended, retrying connection in {}s",
                    backoff_delay.as_secs()
                );
                sleep(backoff_delay).await;
                backoff_delay = (backoff_delay * 2).min(Duration::from_secs(MAX_BACKOFF_SECS));
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }
}

fn extract_event(buffer: &mut Vec<u8>) -> Option<String> {
    fn find_delim(buf: &[u8], pattern: &[u8]) -> Option<usize> {
        buf.windows(pattern.len())
            .position(|window| window == pattern)
    }

    let (event_len, delim_len) = if let Some(pos) = find_delim(buffer, b"\r\n\r\n") {
        (pos, 4)
    } else if let Some(pos) = find_delim(buffer, b"\n\n") {
        (pos, 2)
    } else {
        return None;
    };

    let event_bytes: Vec<u8> = buffer.drain(..event_len).collect();
    buffer.drain(..delim_len);

    Some(String::from_utf8_lossy(&event_bytes).into_owned())
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn mevshare_collector() {
        let collector = MevShareCollector::new("https://mev-share.flashbots.net".into());
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
