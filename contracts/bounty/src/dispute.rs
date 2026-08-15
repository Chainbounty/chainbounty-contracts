use soroban_sdk::{Address, Env};

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
