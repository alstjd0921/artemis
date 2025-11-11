use anyhow::Result;
use clap::Parser;

/// CLI Options.
#[derive(Parser, Debug)]
pub struct Args {
    /// Optional flag for future extensions.
    #[arg(long)]
    pub dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.dry_run {
        println!("Opensea Sudo Arb strategy has been removed; dry-run flag has no effect.");
    } else {
        println!("Opensea Sudo Arb strategy has been removed from Artemis.");
    }
    Ok(())
}
