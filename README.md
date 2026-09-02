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
# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Or use the build script (includes optimization)
chmod +x scripts/build-contracts.sh
./scripts/build-contracts.sh
```

The build script produces an optimized WASM file at:
`target/wasm32-unknown-unknown/release/chainbounty_optimized.wasm`

## Test

```bash
cargo test
```

## Deploy

See [DEPLOYMENTS.md](DEPLOYMENTS.md) for deployment history and contract addresses.

### Testnet Deployment

```bash
# Prerequisites:
# 1. Install soroban-cli: cargo install --locked soroban-cli --features opt
# 2. Configure testnet: soroban network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"
# 3. Generate identity: soroban keys generate deployer
# 4. Fund account: https://laboratory.stellar.org/#account-creator?network=test

chmod +x scripts/deploy-testnet.sh
./scripts/deploy-testnet.sh
```

After deployment, initialize the contract:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source deployer \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --fee_bps 500
```

### Mainnet Deployment

⚠️ **Only deploy to mainnet after:**
- Complete security audit
- Extensive testnet testing
- Multi-sig or governance setup for admin role

```bash
chmod +x scripts/deploy-mainnet.sh
./scripts/deploy-mainnet.sh
```

## Contract ABI Reference

### Data Types

#### BountyStatus

```rust
enum BountyStatus {
    Open,       // Accepting claims
    Claimed,    // Contributor locked in, working
    Submitted,  // Work submitted, awaiting review
    Completed,  // Approved and paid out
    Disputed,   // Dispute raised
    Resolved,   // Dispute resolved by admin
    Cancelled,  // Cancelled by poster before claim
}
```

#### Bounty

```rust
struct Bounty {
    id: u64,                    // Auto-incremented unique ID
    poster: Address,            // Bounty creator
    token: Address,             // Token contract address (XLM or asset)
    amount: i128,               // Escrow amount in token's smallest unit
    title: String,              // Bounty title (min 3 chars)
    description_hash: String,   // IPFS CID or content hash
    deadline: u64,              // Unix timestamp deadline
    status: BountyStatus,       // Current lifecycle state
    contributor: Option<Address>, // Locked contributor (if claimed)
    work_hash: Option<String>,  // IPFS CID of submitted work
    created_at: u64,            // Ledger timestamp of creation
    updated_at: u64,            // Last state transition timestamp
}
```

#### ContractError

```rust
enum ContractError {
    AlreadyInitialized,    // Contract already initialized
    NotInitialized,        // Contract not initialized
    Unauthorized,          // Caller lacks permission
    BountyNotFound,        // Invalid bounty ID
    InvalidStatus,         // Operation invalid for current status
    InvalidAmount,         // Zero or negative amount
    InvalidDeadline,       // Deadline in the past
    InvalidFee,            // Fee > 10% (1000 bps)
    InvalidInput,          // Empty or too-short string
    DeadlineExpired,       // Deadline has passed
    InvalidSplit,          // Split percentage > 100
    NotAParty,             // Caller not poster or contributor
}
```

### Entry Points

#### `initialize(admin: Address, fee_bps: u32) -> Result<(), ContractError>`

Initialize the contract (one-time only).

- **admin:** Address with dispute resolution privileges
- **fee_bps:** Platform fee in basis points (max 1000 = 10%)
- **Emits:** `init` event

#### `post_bounty(poster: Address, token: Address, amount: i128, title: String, description_hash: String, deadline: u64) -> Result<u64, ContractError>`

Create a new bounty with escrowed funds.

- **Returns:** Bounty ID
- **Requires:** Poster must pre-approve token transfer
- **Emits:** `posted` event
- **Errors:** `InvalidAmount`, `InvalidDeadline`, `InvalidInput`, `NotInitialized`

#### `claim_bounty(contributor: Address, bounty_id: u64) -> Result<(), ContractError>`

Lock a contributor onto an Open bounty.

- **Emits:** `claimed` event
- **Errors:** `BountyNotFound`, `InvalidStatus`, `DeadlineExpired`, `Unauthorized` (self-claim)

#### `submit_work(contributor: Address, bounty_id: u64, work_hash: String) -> Result<(), ContractError>`

Submit work as proof of completion.

- **work_hash:** IPFS CID or content hash
- **Emits:** `submitted` event
- **Errors:** `InvalidStatus`, `Unauthorized`, `DeadlineExpired`, `InvalidInput`

#### `approve_submission(poster: Address, bounty_id: u64) -> Result<(), ContractError>`

Approve work and release escrow to contributor (minus platform fee).

- **Transfers:**
  - Contributor receives `amount * (10000 - fee_bps) / 10000`
  - Admin receives platform fee
- **Increments:** Contributor reputation by 1
- **Emits:** `approved` event
- **Errors:** `InvalidStatus`, `Unauthorized`

#### `reject_submission(poster: Address, bounty_id: u64) -> Result<(), ContractError>`

Reject work and reset bounty to Open (escrow remains locked).

- **Emits:** `rejected` event
- **Errors:** `InvalidStatus`, `Unauthorized`

#### `cancel_bounty(poster: Address, bounty_id: u64) -> Result<(), ContractError>`

Cancel an Open bounty and refund full escrow to poster.

- **Emits:** `cancelled` event
- **Errors:** `InvalidStatus` (can only cancel Open bounties), `Unauthorized`

#### `dispute_bounty(caller: Address, bounty_id: u64) -> Result<(), ContractError>`

Raise a dispute on a Submitted bounty (poster or contributor only).

- **Emits:** `disputed` event
- **Errors:** `InvalidStatus`, `NotAParty`

#### `resolve_dispute(resolver: Address, bounty_id: u64, contributor_pct: u32) -> Result<(), ContractError>`

Admin resolves dispute with a split payment.

- **contributor_pct:** Percentage (0–100) awarded to contributor
- **Transfers:**
  - Contributor receives `(amount * contributor_pct / 100) * (10000 - fee_bps) / 10000`
  - Admin receives platform fee on contributor's share
  - Poster receives remainder
- **Emits:** `resolved` event
- **Errors:** `InvalidSplit`, `InvalidStatus`, `Unauthorized` (admin only)

#### `get_bounty(bounty_id: u64) -> Result<Bounty, ContractError>`

Read a single bounty by ID.

#### `list_bounties(from_id: u64, limit: u32) -> Vec<Bounty>`

Paginated read of bounties (max 20 per call).

#### `get_reputation(contributor: Address) -> u32`

Read contributor's reputation score (completed bounties count).

#### `bounty_count() -> u64`

Total number of bounties created.

## Event Schema

All events use Soroban's native event system with short symbol topics (≤9 chars).

### `init`

Contract initialized.

```rust
topic: (symbol_short!("init"), admin: Address)
data: fee_bps: u32
```

### `posted`

New bounty created.

```rust
topic: (symbol_short!("posted"), bounty_id: u64)
data: (poster: Address, amount: i128)
```

### `claimed`

Bounty claimed by contributor.

```rust
topic: (symbol_short!("claimed"), bounty_id: u64)
data: contributor: Address
```

### `submitted`

Work submitted.

```rust
topic: (symbol_short!("submitted"), bounty_id: u64)
data: (contributor: Address, work_hash: String)
```

### `approved`

Work approved, escrow released.

```rust
topic: (symbol_short!("approved"), bounty_id: u64)
data: (contributor: Address, payout: i128)
```

### `rejected`

Work rejected, bounty reset to Open.

```rust
topic: (symbol_short!("rejected"), bounty_id: u64)
data: poster: Address
```

### `cancelled`

Bounty cancelled, escrow refunded.

```rust
topic: (symbol_short!("cancelled"), bounty_id: u64)
data: poster: Address
```

### `disputed`

Dispute raised.

```rust
topic: (symbol_short!("disputed"), bounty_id: u64)
data: caller: Address
```

### `resolved`

Dispute resolved by admin.

```rust
topic: (symbol_short!("resolved"), bounty_id: u64)
data: (resolver: Address, contributor_pct: u32)
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
