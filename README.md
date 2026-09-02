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
      bounty.rs       # core bounty logic (post, claim, submit, approve, reject, cancel)
      dispute.rs      # dispute resolution (dispute, resolve)
      validation.rs   # shared validation helpers (extracted for code quality)
      events.rs       # on-chain event definitions
      tests.rs        # unit tests
    Cargo.toml
Cargo.toml            # workspace root
rust-toolchain.toml   # pinned Rust toolchain
```

### Validation Module

The `validation.rs` module encapsulates all validation logic in reusable functions:
- **Input validation:** amounts, deadlines, strings, percentages
- **Time-based guards:** deadline expiry checks
- **Fee logic:** platform fee reading and split calculations

This separation improves code quality by:
- Providing a single source of truth for validation rules
- Making validation logic testable in isolation
- Keeping state transition code focused and readable

## Dependency Hygiene

### Pinned Versions

- **Rust toolchain:** 1.81.0 (see `rust-toolchain.toml`)
- **Soroban SDK:** 21.7.6 (see `Cargo.toml`)

These versions are pinned to ensure reproducible builds and prevent breaking changes from upstream dependencies.

### Security Notes

**⚠️ This contract has NOT been audited. Use at your own risk.**

Key security considerations for production deployments:

1. **Admin Key Management**
   - The admin address has privileged access (dispute resolution, fee collection)
   - Use multi-sig or governance contract for admin in production
   - Admin cannot modify existing bounties, only resolve disputes

2. **Integer Arithmetic**
   - All fee calculations use integer division (truncates in favor of payee)
   - Amounts are in token's smallest unit (stroops for XLM)
   - No overflow guards beyond Rust's default behavior

3. **Token Approvals**
   - Posters must pre-approve token transfers before calling `post_bounty`
   - Contract does not validate token contract behavior
   - Use only trusted Stellar Asset Contracts

4. **Deadline Enforcement**
   - Deadlines are enforced at claim/submit time only
   - Expired bounties remain in storage; poster must cancel for refund
   - No automatic cleanup of expired bounties

5. **Dispute Resolution**
   - Admin has full discretion over split percentage (0–100%)
   - No time limits on dispute resolution
   - Platform fee is always deducted from contributor's share

6. **Reputation System**
   - Reputation increments on approval only (not on dispute resolution)
   - No decay or time-weighting
   - Reputation is informational only; contract does not enforce minimum scores

### Recommended Audits

Before mainnet deployment:
- Smart contract security audit (logic vulnerabilities, reentrancy, access control)
- Economic model review (fee calculations, edge cases, game theory)
- Integration testing with real Stellar Asset Contracts
- Stress testing with large bounty counts and pagination

## License

Apache-2.0
