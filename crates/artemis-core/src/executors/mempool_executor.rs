use std::sync::Arc;

use crate::types::Executor;
use alloy::network::TransactionBuilder;
use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use anyhow::{Context, Result};
use async_trait::async_trait;

/// An executor that sends transactions to the mempool.
pub struct MempoolExecutor<M> {
    client: Arc<M>,
}

/// Information about the gas bid for a transaction.
#[derive(Debug, Clone)]
pub struct GasBidInfo {
    /// Total profit expected from opportunity
    pub total_profit: U256,

    /// Percentage of bid profit to use for gas
    pub bid_percentage: u64,
}

#[derive(Debug, Clone)]
pub struct SubmitTxToMempool {
    pub tx: TransactionRequest,
    pub gas_bid_info: Option<GasBidInfo>,
}

impl<M: Provider + Send + Sync + 'static> MempoolExecutor<M> {
    pub fn new(client: Arc<M>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<M> Executor<SubmitTxToMempool> for MempoolExecutor<M>
where
    M: Provider + Send + Sync + 'static,
{
    /// Send a transaction to the mempool.
    async fn execute(&self, action: SubmitTxToMempool) -> Result<()> {
        let mut tx = action.tx;
        let gas_usage = U256::from(
            self.client
                .estimate_gas(tx.clone())
                .await
                .context("Error estimating gas usage")?,
        );

        let bid_gas_price = if let Some(gas_bid_info) = action.gas_bid_info {
            // gas price at which we'd break even, meaning 100% of profit goes to validator
            let breakeven_gas_price = gas_bid_info.total_profit / gas_usage;
            // gas price corresponding to bid percentage
            let scaled =
                breakeven_gas_price * U256::from(gas_bid_info.bid_percentage) / U256::from(100u64);
            u128::try_from(scaled).context("bid gas price exceeds u128 range")?
        } else {
            self.client
                .get_gas_price()
                .await
                .context("Error getting gas price")?
        };
        tx = tx.with_gas_price(bid_gas_price);
        let _pending = self.client.send_transaction(tx).await?;
        Ok(())
    }
}
