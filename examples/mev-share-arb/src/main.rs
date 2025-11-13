use std::sync::Arc;

use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use artemis_core::types::{MEV_RELAY, MEV_SHARE};
use artemis_core::{
    collectors::mevshare_collector::MevShareCollector,
    engine::Engine,
    executors::mev_share_executor::MevshareExecutor,
    types::{CollectorMap, ExecutorMap},
};
use clap::Parser;
use mev_share_uni_arb::{
    strategy::MevShareUniArb,
    types::{Action, Event},
};
use tracing::{Level, info};
use tracing_subscriber::{filter, prelude::*};

/// CLI Options.
#[derive(Parser, Debug)]
pub struct Args {
    /// Ethereum node WS endpoint.
    #[arg(long)]
    pub wss: String,
    /// Private key for sending txs.
    #[arg(long)]
    pub private_key: String,
    /// MEV share signer.
    #[arg(long)]
    pub flashbots_signer: String,
    /// Address of the arb contract.
    #[arg(long)]
    pub arb_contract_address: Address,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up tracing and parse args.
    let filter = filter::Targets::new()
        .with_target("mev_share_uni_arb", Level::INFO)
        .with_target("artemis_core", Level::INFO);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let args = Args::parse();

    // Set up provider and signers.
    let ws = WsConnect::new(args.wss.clone());
    let provider = Arc::new(
        ProviderBuilder::new()
            .connect_ws(ws)
            .await
            .context("failed to connect websocket provider")?,
    );

    let wallet_signer: PrivateKeySigner = args
        .private_key
        .parse()
        .context("failed to parse private key")?;
    let wallet = EthereumWallet::from(wallet_signer);

    let mev_provider = Arc::new(
        ProviderBuilder::new().connect_http(MEV_RELAY.parse().expect("failed to parse relay url")),
    );
    let fb_signer: PrivateKeySigner = args
        .flashbots_signer
        .parse()
        .context("failed to parse flashbots signer key")?;

    // Set up engine.
    let mut engine: Engine<Event, Action> = Engine::default();

    // Set up collector.
    let mevshare_collector = Box::new(MevShareCollector::new());
    let mevshare_collector = CollectorMap::new(mevshare_collector, Event::MEVShareEvent);
    engine.add_collector(Box::new(mevshare_collector));

    // Set up strategy.
    let strategy = MevShareUniArb::new(provider.clone(), wallet.clone(), args.arb_contract_address);
    engine.add_strategy(Box::new(strategy));

    // Set up executor.
    let mev_share_executor = Box::new(MevshareExecutor::new(mev_provider, fb_signer));
    let mev_share_executor = ExecutorMap::new(mev_share_executor, |action| match action {
        Action::SubmitBundle(bundle) => Some(bundle),
    });
    engine.add_executor(Box::new(mev_share_executor));

    // Start engine.
    if let Ok(mut set) = engine.run().await {
        while let Some(res) = set.join_next().await {
            info!("res: {:?}", res);
        }
    }

    Ok(())
}
