# chainbounty-contracts

Soroban smart contracts for the ChainBounty protocol — a decentralized bounty board for open-source contributors built on Stellar.

## Overview

ChainBounty contracts handle the full bounty lifecycle on-chain:
- Posting a bounty with XLM/token escrow
- Claiming a bounty by a contributor
- Submitting work (IPFS hash)
- Approving/rejecting submissions with automatic escrow release
- Dispute resolution with configurable split payments
- Platform fee deduction
- Contributor reputation scoring

## Prerequisites

- [Rust](https://rustup.rs/) (toolchain pinned via `rust-toolchain.toml`)
- [Soroban CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/soroban-cli)
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked soroban-cli --features opt
```

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test
```

## Project Structure

```
contracts/
  bounty/
    src/
      lib.rs          # contract entry point
      types.rs        # data types and enums
      bounty.rs       # core bounty logic
      dispute.rs      # dispute resolution
      validation.rs   # shared validation helpers
      events.rs       # on-chain event definitions
    Cargo.toml
Cargo.toml            # workspace root
rust-toolchain.toml   # pinned Rust toolchain
```

## License

Apache-2.0
