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

    crate::events::emit_bounty_posted(&env, id, &poster, amount);

    Ok(id)
}

// ---------------------------------------------------------------------------
// claim_bounty
// ---------------------------------------------------------------------------

/// Lock a contributor onto an Open bounty.
///
/// Rules enforced:
/// - Bounty must exist and be in `Open` status.
/// - Deadline must not have passed.
/// - Poster cannot claim their own bounty.
pub fn claim_bounty(
    env: Env,
    contributor: Address,
    bounty_id: BountyId,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: must be Open
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::InvalidStatus);
    }

    // Deadline guard: must not have expired
    validation::require_not_expired(&env, bounty.deadline)?;

    // Self-claim guard: poster cannot be the contributor
    if bounty.poster == contributor {
        return Err(ContractError::Unauthorized);
    }

    // Lock the contributor and advance status
    bounty.contributor = Some(contributor.clone());
    bounty.status = BountyStatus::Claimed;
    save_bounty(&env, bounty);

    crate::events::emit_bounty_claimed(&env, bounty_id, &contributor);

    Ok(())
}

// ---------------------------------------------------------------------------
// submit_work
// ---------------------------------------------------------------------------

/// Contributor submits an IPFS content hash as proof of work.
///
/// Rules enforced:
/// - Bounty must exist and be in `Claimed` status.
/// - Caller must be the locked contributor.
/// - Deadline must not have passed.
/// - Work hash must be a non-empty string.
pub fn submit_work(
    env: Env,
    contributor: Address,
    bounty_id: BountyId,
    work_hash: String,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: must be Claimed
    if bounty.status != BountyStatus::Claimed {
        return Err(ContractError::InvalidStatus);
    }

    // Ownership guard: only the locked contributor may submit
    match &bounty.contributor {
        Some(c) if *c == contributor => {}
        _ => return Err(ContractError::Unauthorized),
    }

    // Deadline guard
    validation::require_not_expired(&env, bounty.deadline)?;

    // Input guard: work hash must be non-empty
    validation::require_non_empty_string(&work_hash)?;

    // Store work hash and advance status
    bounty.work_hash = Some(work_hash.clone());
    bounty.status = BountyStatus::Submitted;
    save_bounty(&env, bounty);

    crate::events::emit_work_submitted(&env, bounty_id, &contributor, &work_hash);

    Ok(())
}

// ---------------------------------------------------------------------------
// cancel_bounty
// ---------------------------------------------------------------------------

/// Poster cancels an Open bounty and receives a full escrow refund.
///
/// Rules enforced:
/// - Bounty must be in `Open` status (no contributor locked in).
/// - Caller must be the original poster.
/// - Full escrow amount is returned to the poster.
pub fn cancel_bounty(
    env: Env,
    poster: Address,
    bounty_id: BountyId,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: can only cancel an Open bounty
    if bounty.status != BountyStatus::Open {
        return Err(ContractError::InvalidStatus);
    }

    // Ownership guard: only the poster may cancel
    if bounty.poster != poster {
        return Err(ContractError::Unauthorized);
    }

    // Refund full escrow to poster
    let token_client = token::Client::new(&env, &bounty.token);
    let contract_addr = env.current_contract_address();
    token_client.transfer(&contract_addr, &poster, &bounty.amount);

    // Mark as Cancelled
    bounty.status = BountyStatus::Cancelled;
    save_bounty(&env, bounty);

    crate::events::emit_cancelled(&env, bounty_id, &poster);

    Ok(())
}

// ---------------------------------------------------------------------------
// reject_submission
// ---------------------------------------------------------------------------

/// Poster rejects submitted work — clears contributor and resets bounty to Open.
///
/// Rules enforced:
/// - Bounty must be in `Submitted` status.
/// - Caller must be the original poster.
/// - Escrow stays locked; bounty re-opens for a new contributor.
pub fn reject_submission(
    env: Env,
    poster: Address,
    bounty_id: BountyId,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: must be Submitted
    if bounty.status != BountyStatus::Submitted {
        return Err(ContractError::InvalidStatus);
    }

    // Ownership guard: only the poster may reject
    if bounty.poster != poster {
        return Err(ContractError::Unauthorized);
    }

    // Clear contributor and work hash, reset to Open
    bounty.contributor = None;
    bounty.work_hash = None;
    bounty.status = BountyStatus::Open;
    save_bounty(&env, bounty);

    crate::events::emit_rejected(&env, bounty_id, &poster);

    Ok(())
}

// ---------------------------------------------------------------------------
// approve_submission
// ---------------------------------------------------------------------------

/// Poster approves the submitted work and releases escrow to the contributor.
///
/// Rules enforced:
/// - Bounty must be in `Submitted` status.
/// - Caller must be the original poster.
/// - Platform fee is deducted; remainder goes to contributor.
/// - Fee portion is transferred to the contract admin address.
pub fn approve_submission(
    env: Env,
    poster: Address,
    bounty_id: BountyId,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: must be Submitted
    if bounty.status != BountyStatus::Submitted {
        return Err(ContractError::InvalidStatus);
    }

    // Ownership guard: only the poster may approve
    if bounty.poster != poster {
        return Err(ContractError::Unauthorized);
    }

    // Resolve contributor (always present when Submitted)
    let contributor = bounty.contributor.clone().ok_or(ContractError::Unauthorized)?;

    // Calculate payout split
    let fee_bps = validation::get_fee_bps(&env);
    let (payout, fee) = validation::compute_fee_split(bounty.amount, fee_bps);

    let token_client = token::Client::new(&env, &bounty.token);
    let contract_addr = env.current_contract_address();

    // Release reward to contributor
    token_client.transfer(&contract_addr, &contributor, &payout);

    // Send fee to admin (only if non-zero)
    if fee > 0 {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        token_client.transfer(&contract_addr, &admin, &fee);
    }

    // Advance status to Completed
    bounty.status = BountyStatus::Completed;
    save_bounty(&env, bounty);

    // Increment contributor reputation
    increment_reputation(&env, &contributor);

    crate::events::emit_approved(&env, bounty_id, &contributor, payout);

    Ok(())
}

// ---------------------------------------------------------------------------
// Reputation helpers
// ---------------------------------------------------------------------------

/// Increment a contributor's reputation score by 1.
fn increment_reputation(env: &Env, contributor: &Address) {
    let current: u32 = env
        .storage()
        .persistent()
        .get(&DataKey::Reputation(contributor.clone()))
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&DataKey::Reputation(contributor.clone()), &(current + 1));
}

/// Read a contributor's reputation score (public view function).
pub fn get_reputation(env: &Env, contributor: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::Reputation(contributor.clone()))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// get_bounty / list_bounties (read-only)
// ---------------------------------------------------------------------------

/// Return a single bounty by ID.
pub fn get_bounty(env: &Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
    load_bounty(env, bounty_id)
}

/// Return a page of bounties in ascending ID order.
///
/// - `from_id`  — first bounty ID to include (1-based; pass 1 to start from the beginning)
/// - `limit`    — maximum number of records to return (capped at 20 internally)
///
/// IDs that no longer exist in storage are silently skipped.
pub fn list_bounties(env: &Env, from_id: BountyId, limit: u32) -> soroban_sdk::Vec<Bounty> {
    let cap: u32 = if limit > 20 { 20 } else { limit };
    let total: u64 = env
        .storage()
        .instance()
        .get(&DataKey::BountyCount)
        .unwrap_or(0);

    let mut results: soroban_sdk::Vec<Bounty> = soroban_sdk::Vec::new(env);
    let mut collected: u32 = 0;
    let mut id = from_id;

    while id <= total && collected < cap {
        if let Some(b) = env
            .storage()
            .persistent()
            .get::<DataKey, Bounty>(&DataKey::Bounty(id))
        {
            results.push_back(b);
            collected += 1;
        }
        id += 1;
    }

    results
}
