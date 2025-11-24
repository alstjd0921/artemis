//! Executors are responsible for taking actions produced by strategies and
//! executing them in different domains. For example, an executor might take a
//! `SubmitTx` action and submit it to the mempool.

/// This executor submits transactions to the flashbots relay.
pub mod flashbots_executor;

/// This executor submits transactions to the public mempool.
pub mod mempool_executor;

/// This executor submits bundles to the flashbots matchmaker.
pub mod mev_share_executor;

/// This executor submits private fast transactions to flashbots.
pub mod flashbots_single_executor;
