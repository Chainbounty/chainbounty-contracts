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
    bounty.contributor = Some(contributor);
    bounty.status = BountyStatus::Claimed;
    save_bounty(&env, bounty);

    Ok(())
}

// ---------------------------------------------------------------------------
// helpers: fee & payout
// ---------------------------------------------------------------------------

/// Read the platform fee basis points from storage (defaults to 0 if unset).
fn get_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::FeeBps)
        .unwrap_or(0u32)
}

/// Calculate (contributor_payout, platform_fee) from a gross amount and fee bps.
/// Uses integer arithmetic; truncates fractional stroops in favour of contributor.
pub fn split_payout(amount: i128, fee_bps: u32) -> (i128, i128) {
    let fee = amount * fee_bps as i128 / 10_000;
    let payout = amount - fee;
    (payout, fee)
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
    bounty.work_hash = Some(work_hash);
    bounty.status = BountyStatus::Submitted;
    save_bounty(&env, bounty);

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
    let fee_bps = get_fee_bps(&env);
    let (payout, fee) = split_payout(bounty.amount, fee_bps);

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

    Ok(())
}

// ---------------------------------------------------------------------------
// get_bounty (read-only)
// ---------------------------------------------------------------------------

pub fn get_bounty(env: &Env, bounty_id: BountyId) -> Result<Bounty, ContractError> {
    load_bounty(env, bounty_id)
}
