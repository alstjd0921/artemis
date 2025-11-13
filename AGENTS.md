# Repository Guidelines

## Project Structure & Module Organization
Artemis is a Cargo workspace that joins runnable bins under `bin/` with reusable crates in `crates/`. `bin/artemis` owns the event-pipeline orchestrator, `bin/cli` exposes the generator, `crates/artemis-core` carries shared traits, and `crates/generator` plus `crates/strategies/mev-share-uni-arb` supply scaffolding and example logic. Reference bots live under `examples/` (start with `mev-share-arb`), visuals in `assets/`, and Docker entry points in `docker/`. Keep generated bindings or Foundry sources at the root so `just` tasks discover them.

## Build, Test, and Development Commands
- `cargo check --workspace` – fast compile guard across all members.
- `cargo test --all` – runs every unit and integration test in the workspace.
- `cargo run -p artemis -- --help` (or `-p mev-share-arb`) – inspect runtime flags or launch a sample strategy.
- `just fmt` – runs `cargo +nightly fmt --all`.
- `just clippy` – runs `cargo clippy --all --all-features`.
- `just build-contracts` / `just test-contracts` – install and test any Foundry contracts placed under `contracts/`.
- `just build-bindings-crate` – regenerate `bindings/` via `forge bind`.

## Coding Style & Naming Conventions
Use standard 4-space indentation, keep modules and files in `snake_case`, types and traits in `CamelCase`, and CLI flags in kebab-case. Run `just fmt` before committing; `rustfmt.toml` ensures generated `bindings/` stay untouched, so avoid manual edits there. Organize collectors, strategies, and executors by domain directory (e.g., `collectors/mevshare_collector.rs`) and add short doc comments describing the MEV surface they cover.

## Testing Guidelines
`cargo test --all` is the baseline gate and async components should lean on `#[tokio::test]` for deterministic runtimes. New strategies require integration tests in the crate’s `tests/` folder that assert collector→strategy→executor behavior plus error cases. Solidity helpers must keep `forge test --root ./contracts` (or `just test-contracts`) green; note skipped network calls inline.

## Commit & Pull Request Guidelines
Match the existing `type: summary` format (`fix:`, `chore:`, `feat:`) and keep subjects imperative and under 72 characters. PRs should explain the MEV opportunity or infra touched and list new configs or CLI flags plus relevant issues. Attach logs, bundle IDs, or screenshots when behavior changes, and ensure both Cargo and Foundry jobs pass before requesting review.

## Security & Configuration Tips
Keep secrets such as `ETHERSCAN_API_KEY`, RPC URLs, or MEV-Share tokens inside a local `.env`; the `justfile` autoloads it through `dotenv-load`. Never commit keys or raw bundle payloads; prefer redacted hashes and dry-run collectors before enabling live executors.

## prompt
Think in English, but always write the last change summary output in Korean.