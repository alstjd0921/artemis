use alloy::primitives::{Address, B256, Bytes};
use serde::{Deserialize, Serialize};

/// SSE event emitted by the MEV-share endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Transaction or bundle hash.
    pub hash: B256,
    /// Transactions referenced by the event.
    #[serde(default, rename = "txs")]
    pub transactions: Vec<EventTransaction>,
    /// Logs emitted while executing the event transaction.
    #[serde(default)]
    pub logs: Vec<EventTransactionLog>,
}

/// Transaction metadata carried in MEV-share events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTransaction {
    /// Recipient address.
    pub to: Option<Address>,
    /// 4-byte function selector as hex string.
    #[serde(rename = "functionSelector", default)]
    pub function_selector: Option<String>,
    /// Raw calldata.
    #[serde(rename = "callData", default)]
    pub calldata: Option<Bytes>,
}

/// Log metadata carried in MEV-share events.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventTransactionLog {
    pub address: Address,
    #[serde(default)]
    pub topics: Vec<B256>,
}
