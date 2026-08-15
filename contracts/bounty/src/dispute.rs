use soroban_sdk::{token, Address, Env};

use crate::types::{BountyId, BountyStatus, ContractError, DataKey};

// ---------------------------------------------------------------------------
// Helpers (local to dispute module)
// ---------------------------------------------------------------------------

fn load_bounty(
    env: &Env,
    bounty_id: BountyId,
) -> Result<crate::types::Bounty, ContractError> {
    env.storage()
        .persistent()
        .get(&DataKey::Bounty(bounty_id))
        .ok_or(ContractError::BountyNotFound)
}

fn save_bounty(env: &Env, mut bounty: crate::types::Bounty) -> crate::types::Bounty {
    bounty.updated_at = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&DataKey::Bounty(bounty.id), &bounty);
    bounty
}

// ---------------------------------------------------------------------------
// dispute_bounty
// ---------------------------------------------------------------------------

/// Either party (poster or contributor) raises a dispute on a Submitted bounty.
///
/// Rules enforced:
/// - Bounty must be in `Submitted` status.
/// - Caller must be either the poster or the locked contributor.
/// - Transitions status to `Disputed`; escrow stays locked pending resolution.
pub fn dispute_bounty(
    env: Env,
    caller: Address,
    bounty_id: BountyId,
) -> Result<(), ContractError> {
    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: disputes are only valid on Submitted bounties
    if bounty.status != BountyStatus::Submitted {
        return Err(ContractError::InvalidStatus);
    }

    // Party guard: caller must be poster or contributor
    let is_poster = bounty.poster == caller;
    let is_contributor = bounty
        .contributor
        .as_ref()
        .map(|c| *c == caller)
        .unwrap_or(false);

    if !is_poster && !is_contributor {
        return Err(ContractError::NotAParty);
    }

    // Advance to Disputed
    bounty.status = BountyStatus::Disputed;
    save_bounty(&env, bounty);

    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_dispute
// ---------------------------------------------------------------------------

/// Admin resolves a disputed bounty by splitting the escrow.
///
/// `contributor_pct` is the percentage (0–100) that goes to the contributor.
/// The remainder goes back to the poster.
/// Platform fee is deducted from the contributor's portion before transfer.
///
/// Rules enforced:
/// - Bounty must be in `Disputed` status.
/// - Caller must be the stored admin address.
/// - contributor_pct must be between 0 and 100 (inclusive).
pub fn resolve_dispute(
    env: Env,
    resolver: Address,
    bounty_id: BountyId,
    contributor_pct: u32,
) -> Result<(), ContractError> {
    // Validate split percentage
    crate::validation::require_valid_split(contributor_pct)?;

    let mut bounty = load_bounty(&env, bounty_id)?;

    // Status guard: must be Disputed
    if bounty.status != BountyStatus::Disputed {
        return Err(ContractError::InvalidStatus);
    }

    // Admin guard: only the stored admin may resolve
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(ContractError::NotInitialized)?;

    if admin != resolver {
        return Err(ContractError::Unauthorized);
    }

    let contributor = bounty
        .contributor
        .clone()
        .ok_or(ContractError::Unauthorized)?;

    let token_client = token::Client::new(&env, &bounty.token);
    let contract_addr = env.current_contract_address();

    // --- calculate shares ---
    // contributor gross share
    let contributor_gross = bounty.amount * contributor_pct as i128 / 100;
    // poster refund (remainder)
    let poster_refund = bounty.amount - contributor_gross;

    // deduct platform fee from contributor's gross share
    let fee_bps: u32 = env
        .storage()
        .instance()
        .get(&DataKey::FeeBps)
        .unwrap_or(0u32);

    let fee = contributor_gross * fee_bps as i128 / 10_000;
    let contributor_net = contributor_gross - fee;

    // --- transfers ---
    if contributor_net > 0 {
        token_client.transfer(&contract_addr, &contributor, &contributor_net);
    }

    if fee > 0 {
        token_client.transfer(&contract_addr, &admin, &fee);
    }

    if poster_refund > 0 {
        token_client.transfer(&contract_addr, &bounty.poster, &poster_refund);
    }

    // Advance to Resolved
    bounty.status = BountyStatus::Resolved;
    save_bounty(&env, bounty);

    Ok(())
}
