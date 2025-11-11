use alloy::primitives::Address;
use alloy::rpc::types::mev::MevSendBundle;
use artemis_core::mevshare;

/// Core Event enum for the current strategy.
#[derive(Debug, Clone)]
pub enum Event {
    MEVShareEvent(mevshare::Event),
}

/// Core Action enum for the current strategy.
#[derive(Debug, Clone)]
pub enum Action {
    SubmitBundle(MevSendBundle),
}

#[derive(Debug, serde::Deserialize)]
pub struct PoolRecord {
    pub token_address: Address,
    pub uni_pool_address: Address,
    pub sushi_pool_address: Address,
}

#[derive(Debug, serde::Deserialize)]
pub struct V2V3PoolRecord {
    pub token_address: Address,
    pub v3_pool: Address,
    pub v2_pool: Address,
    pub weth_token0: bool,
}
