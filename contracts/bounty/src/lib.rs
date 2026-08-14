#![no_std]

mod bounty;
mod types;
mod validation;

use soroban_sdk::{contract, contractimpl, Address, Env, String};

use types::{BountyId, DataKey};

pub use types::{Bounty, BountyStatus, ContractError};

/// ChainBounty — decentralized on-chain bounty board for open-source contributors.
///
/// Entry points (so far):
///   initialize   — set admin and platform fee
///   post_bounty  — poster locks escrow and creates a bounty
///   get_bounty   — read a single bounty by id
///   bounty_count — total bounties created
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

    /// Read a single bounty by ID.
    pub fn get_bounty(env: Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
        bounty::get_bounty(&env, bounty_id)
    }

    /// Read current bounty count.
    pub fn bounty_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::BountyCount)
            .unwrap_or(0)
    }
}
