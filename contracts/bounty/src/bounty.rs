use soroban_sdk::{token, Address, Env, String};

use crate::types::{Bounty, BountyId, BountyStatus, ContractError, DataKey};
use crate::validation;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load a bounty or return BountyNotFound.
fn load_bounty(env: &Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
    env.storage()
        .persistent()
        .get(&DataKey::Bounty(bounty_id))
        .ok_or(ContractError::BountyNotFound)
}

/// Persist a bounty back to storage and bump its updated_at timestamp.
fn save_bounty(env: &Env, mut bounty: Bounty) -> Bounty {
    bounty.updated_at = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&DataKey::Bounty(bounty.id), &bounty);
    bounty
}

/// Increment and return the next bounty ID.
fn next_bounty_id(env: &Env) -> BountyId {
    let count: u64 = env
        .storage()
        .instance()
        .get(&DataKey::BountyCount)
        .unwrap_or(0);
    let next = count + 1;
    env.storage()
        .instance()
        .set(&DataKey::BountyCount, &next);
    next
}

// ---------------------------------------------------------------------------
// post_bounty
// ---------------------------------------------------------------------------

/// Create a new bounty and transfer the escrow amount from the poster.
///
/// The poster must have called `token.approve(poster, contract, amount, ledger)`
/// before this entry point so the contract can pull the funds.
pub fn post_bounty(
    env: Env,
    poster: Address,
    token: Address,
    amount: i128,
    title: String,
    description_hash: String,
    deadline: u64,
) -> Result<BountyId, ContractError> {
    // --- guard: contract must be initialized ---
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(ContractError::NotInitialized);
    }

    // --- validate inputs ---
    validation::require_positive_amount(amount)?;
    validation::require_future_deadline(&env, deadline)?;
    validation::require_non_empty_string(&title)?;
    validation::require_non_empty_string(&description_hash)?;

    // --- pull escrow from poster ---
    let contract_addr = env.current_contract_address();
    let token_client = token::Client::new(&env, &token);
    token_client.transfer(&poster, &contract_addr, &amount);

    // --- build and store bounty ---
    let id = next_bounty_id(&env);
    let now = env.ledger().timestamp();

    let bounty = Bounty {
        id,
        poster: poster.clone(),
        token,
        amount,
        title,
        description_hash,
        deadline,
        status: BountyStatus::Open,
        contributor: None,
        work_hash: None,
        created_at: now,
        updated_at: now,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Bounty(id), &bounty);

    Ok(id)
}

// ---------------------------------------------------------------------------
// get_bounty (read-only)
// ---------------------------------------------------------------------------

pub fn get_bounty(env: &Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
    load_bounty(env, bounty_id)
}
