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
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::warn;

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
    async fn get_event_stream(&self) -> Result<CollectorStream<'life0, Event>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let url = self.mevshare_sse_url.clone();
        let client = Client::new();

        tokio::spawn(async move {
            let request = match client.get(&url).send().await {
                Ok(resp) => resp,
                Err(err) => {
                    warn!("failed to connect to MEV-share SSE endpoint: {err}");
                    return;
                }
            };

            let mut stream = request.bytes_stream();
            let mut buffer: Vec<u8> = Vec::new();

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
                                    let _ = tx.send(evt);
                                }
                                Err(err) => {
                                    warn!("failed to parse MEV-share event: {err}");
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }
}

fn extract_event(buffer: &mut Vec<u8>) -> Option<String> {
    fn find_delim(buf: &[u8], pattern: &[u8]) -> Option<usize> {
        buf.windows(pattern.len()).position(|window| window == pattern)
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
