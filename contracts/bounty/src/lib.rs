#![no_std]

mod bounty;
mod dispute;
mod events;
mod types;
mod validation;

#[cfg(test)]
mod tests;

use soroban_sdk::{contract, contractimpl, Address, Env, String};

use types::{BountyId, DataKey};

pub use types::{Bounty, BountyStatus, ContractError};

/// ChainBounty — decentralized on-chain bounty board for open-source contributors.
///
/// Entry points (so far):
///   initialize          — set admin and platform fee
///   post_bounty         — poster locks escrow and creates a bounty
///   claim_bounty        — contributor locks their intent to work
///   submit_work         — contributor submits an IPFS work hash
///   approve_submission  — poster approves work and releases escrow
///   reject_submission   — poster rejects work and resets claim
///   cancel_bounty       — poster cancels an open bounty and gets refund
///   dispute_bounty      — either party raises a dispute
///   resolve_dispute     — admin/resolver settles a dispute with split payment
///   get_bounty          — read a single bounty by id
///   list_bounties       — paginated read of all bounties
///   get_reputation      — read contributor reputation score
///   bounty_count        — total bounties created
#[contract]
pub struct ChainBountyContract;

#[contractimpl]
impl ChainBountyContract {
    /// Initialize the contract with admin and platform fee basis points (max 1000 = 10%).
    pub fn initialize(env: Env, admin: Address, fee_bps: u32) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        validation::require_valid_fee(fee_bps)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        env.storage().instance().set(&DataKey::BountyCount, &0u64);
        events::emit_initialized(&env, &admin, fee_bps);
        Ok(())
    }

    /// Post a new bounty. Caller must have approved token transfer beforehand.
    pub fn post_bounty(
        env: Env,
        poster: Address,
        token: Address,
        amount: i128,
        title: String,
        description_hash: String,
        deadline: u64,
    ) -> Result<BountyId, ContractError> {
        poster.require_auth();
        bounty::post_bounty(env, poster, token, amount, title, description_hash, deadline)
    }

    /// Claim an open bounty. Locks the contributor to this bounty.
    pub fn claim_bounty(env: Env, contributor: Address, bounty_id: BountyId) -> Result<(), ContractError> {
        contributor.require_auth();
        bounty::claim_bounty(env, contributor, bounty_id)
    }

    /// Submit work for a claimed bounty via IPFS content hash.
    pub fn submit_work(
        env: Env,
        contributor: Address,
        bounty_id: BountyId,
        work_hash: String,
    ) -> Result<(), ContractError> {
        contributor.require_auth();
        bounty::submit_work(env, contributor, bounty_id, work_hash)
    }

    /// Poster approves submitted work — releases escrow to contributor minus fee.
    pub fn approve_submission(env: Env, poster: Address, bounty_id: BountyId) -> Result<(), ContractError> {
        poster.require_auth();
        bounty::approve_submission(env, poster, bounty_id)
    }

    /// Poster rejects submitted work — resets bounty back to Open.
    pub fn reject_submission(env: Env, poster: Address, bounty_id: BountyId) -> Result<(), ContractError> {
        poster.require_auth();
        bounty::reject_submission(env, poster, bounty_id)
    }

    /// Poster cancels an Open bounty and receives a full refund.
    pub fn cancel_bounty(env: Env, poster: Address, bounty_id: BountyId) -> Result<(), ContractError> {
        poster.require_auth();
        bounty::cancel_bounty(env, poster, bounty_id)
    }

    /// Either party raises a dispute on a submitted bounty.
    pub fn dispute_bounty(env: Env, caller: Address, bounty_id: BountyId) -> Result<(), ContractError> {
        caller.require_auth();
        dispute::dispute_bounty(env, caller, bounty_id)
    }

    /// Admin/resolver resolves a disputed bounty with a split ratio (0–100 to contributor).
    pub fn resolve_dispute(
        env: Env,
        resolver: Address,
        bounty_id: BountyId,
        contributor_pct: u32,
    ) -> Result<(), ContractError> {
        resolver.require_auth();
        dispute::resolve_dispute(env, resolver, bounty_id, contributor_pct)
    }

    /// Read a single bounty by ID.
    pub fn get_bounty(env: Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
        bounty::get_bounty(&env, bounty_id)
    }

    /// Return a page of bounties in ascending ID order.
    /// `from_id` is the first ID to include; `limit` is max records (capped at 20).
    pub fn list_bounties(env: Env, from_id: BountyId, limit: u32) -> soroban_sdk::Vec<Bounty> {
        bounty::list_bounties(&env, from_id, limit)
    }

    /// Read contributor reputation score (number of completed bounties).
    pub fn get_reputation(env: Env, contributor: Address) -> u32 {
        bounty::get_reputation(&env, &contributor)
    }

    /// Read current bounty count.
    pub fn bounty_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::BountyCount)
            .unwrap_or(0)
    }
}
