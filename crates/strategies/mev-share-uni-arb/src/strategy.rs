use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use alloy::rpc::types::mev::{BundleItem, Inclusion, MevSendBundle, Privacy, ProtocolVersion};
use alloy::{
    eips::Encodable2718,
    network::{Ethereum, NetworkWallet, TransactionBuilder},
    primitives::Bytes,
    primitives::{Address, B256, U256 as AlloyU256},
    providers::Provider,
};
use anyhow::Result;
use artemis_core::types::Strategy;
use async_trait::async_trait;
use tracing::info;

use crate::types::V2V3PoolRecord;

use super::types::{Action, Event};

use mev_share_bindings::blind_arb;

/// Information about an uniswap v2 pool.
#[derive(Debug, Clone)]
pub struct V2PoolInfo {
    /// Address of the v2 pool.
    pub v2_pool: Address,
    /// Whether the pool has weth as token0.
    pub is_weth_token0: bool,
}

#[derive(Debug, Clone)]
pub struct MevShareUniArb<P, W>
where
    P: Provider + Send + Sync + 'static,
    W: NetworkWallet<Ethereum> + Clone + Send + Sync + 'static,
{
    /// Alloy provider used to query on-chain data.
    provider: Arc<P>,
    /// Maps uni v3 pool address to v2 pool information.
    pool_map: HashMap<Address, V2PoolInfo>,
    /// Wallet used for signing transactions.
    wallet: W,
    /// BlindArb contract instance.
    arb_contract: blind_arb::BlindArb::BlindArbInstance<Arc<P>>,
}

impl<P, W> MevShareUniArb<P, W>
where
    P: Provider + Send + Sync + 'static,
    W: NetworkWallet<Ethereum> + Clone + Send + Sync + 'static,
{
    /// Create a new instance of the strategy.
    pub fn new(provider: Arc<P>, wallet: W, arb_contract_address: Address) -> Self {
        let arb_contract = blind_arb::BlindArb::new(arb_contract_address, provider.clone());
        Self {
            provider,
            pool_map: HashMap::new(),
            wallet,
            arb_contract,
        }
    }
}

#[async_trait]
impl<P, W> Strategy<Event, Action> for MevShareUniArb<P, W>
where
    P: Provider + Send + Sync + 'static,
    W: NetworkWallet<Ethereum> + Clone + Send + Sync + 'static,
{
    /// Initialize the strategy. This is called once at startup, and loads
    /// pool information into memory.
    async fn sync_state(&mut self) -> Result<()> {
        // Read pool information from csv file.
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/v3_v2_pools.csv");
        let mut reader = csv::Reader::from_path(path)?;

        for record in reader.deserialize() {
            // Parse records into PoolRecord struct.
            let record: V2V3PoolRecord = record?;
            self.pool_map.insert(
                record.v3_pool,
                V2PoolInfo {
                    v2_pool: record.v2_pool,
                    is_weth_token0: record.weth_token0,
                },
            );
        }

        Ok(())
    }

    // Process incoming events, seeing if we can arb new orders.
    async fn process_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::MEVShareEvent(event) => {
                info!("Received mev share event: {:?}", event);
                // skip if event has no logs
                if event.logs.is_empty() {
                    return vec![];
                }
                let address = event.logs[0].address;
                // skip if address is not a v3 pool
                if !self.pool_map.contains_key(&address) {
                    return vec![];
                }
                // if it's a v3 pool we care about, submit bundles
                info!(
                    "Found a v3 pool match at address {:?}, submitting bundles",
                    address
                );
                self.generate_bundles(address, event.hash)
                    .await
                    .into_iter()
                    .map(Action::SubmitBundle)
                    .collect()
            }
        }
    }
}

impl<P, W> MevShareUniArb<P, W>
where
    P: Provider + Send + Sync + 'static,
    W: NetworkWallet<Ethereum> + Clone + Send + Sync + 'static,
{
    /// Generate a series of bundles of varying sizes to submit to the matchmaker.
    pub async fn generate_bundles(&self, v3_address: Address, tx_hash: B256) -> Vec<MevSendBundle> {
        let mut bundles = Vec::new();
        let v2_info = self.pool_map.get(&v3_address).unwrap();

        // The sizes of the backruns we want to submit.
        // TODO: Run some analysis to figure out likely sizes.
        let sizes = vec![
            AlloyU256::from(100_000_u128),
            AlloyU256::from(1_000_000_u128),
            AlloyU256::from(10_000_000_u128),
            AlloyU256::from(100_000_000_u128),
            AlloyU256::from(1_000_000_000_u128),
            AlloyU256::from(10_000_000_000_u128),
            AlloyU256::from(100_000_000_000_u128),
            AlloyU256::from(1_000_000_000_000_u128),
            AlloyU256::from(10_000_000_000_000_u128),
            AlloyU256::from(100_000_000_000_000_u128),
            AlloyU256::from(1_000_000_000_000_000_u128),
            AlloyU256::from(10_000_000_000_000_000_u128),
            AlloyU256::from(100_000_000_000_000_000_u128),
            AlloyU256::from(1_000_000_000_000_000_000_u128),
        ];

        // Set parameters for the backruns.
        let payment_percentage = AlloyU256::ZERO;
        let bid_gas_price = match self.provider.get_gas_price().await {
            Ok(price) => price,
            Err(err) => {
                info!("Failed to fetch gas price: {err:?}");
                return bundles;
            }
        };
        let block_num = match self.provider.get_block_number().await {
            Ok(number) => number,
            Err(err) => {
                info!("Failed to fetch block number: {err:?}");
                return bundles;
            }
        };
        let chain_id = match self.provider.get_chain_id().await {
            Ok(id) => id,
            Err(err) => {
                info!("Failed to fetch chain id: {err:?}");
                return bundles;
            }
        };
        let sender = self.wallet.default_signer_address();
        let nonce = match self.provider.get_transaction_count(sender).await {
            Ok(value) => value,
            Err(err) => {
                info!("Failed to fetch signer nonce: {err:?}");
                return bundles;
            }
        };

        for size in sizes {
            // Construct arb tx based on whether the v2 pool has weth as token0.
            let mut tx = if v2_info.is_weth_token0 {
                self.arb_contract
                    .executeArb__WETH_token0(v2_info.v2_pool, v3_address, size, payment_percentage)
                    .into_transaction_request()
            } else {
                self.arb_contract
                    .executeArb__WETH_token1(v2_info.v2_pool, v3_address, size, payment_percentage)
                    .into_transaction_request()
            };
            tx.set_from(sender);
            tx.set_nonce(nonce);
            tx.set_chain_id(chain_id);
            tx.set_gas_limit(400_000);
            tx.set_gas_price(bid_gas_price);
            tx.set_value(AlloyU256::ZERO);

            info!("generated arb tx: {:?}", tx);

            let envelope = match tx.clone().build(&self.wallet).await {
                Ok(env) => env,
                Err(err) => {
                    info!("Failed to sign arb transaction: {err:?}");
                    continue;
                }
            };
            let bytes = Bytes::from(envelope.encoded_2718());
            let txs = vec![
                BundleItem::Hash { hash: tx_hash },
                BundleItem::Tx {
                    tx: bytes,
                    can_revert: false,
                },
            ];
            let bundle = MevSendBundle {
                protocol_version: ProtocolVersion::V0_1,
                inclusion: Inclusion {
                    block: block_num + 1,
                    // set a large validity window to ensure builder gets a chance to include bundle.
                    max_block: Some(block_num + 30),
                },
                bundle_body: txs,
                validity: None,
                privacy: Some(Privacy {
                    hints: None,
                    builders: Some(vec![
                        "flashbots".into(),
                        "Titan".into(),
                        "rsync".into(),
                        "beaverbuild.org".into(),
                        "builder0x69".into(),
                        "Quasar".into(),
                    ]),
                }),
            };
            info!("submitting bundle: {:?}", bundle);
            bundles.push(bundle);
        }
        bundles
    }
}
