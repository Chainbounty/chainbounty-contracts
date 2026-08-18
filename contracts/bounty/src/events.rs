use soroban_sdk::{symbol_short, Address, Env, String};

use crate::types::BountyId;

// ---------------------------------------------------------------------------
// Event topics use short symbols (≤ 9 chars) as required by Soroban.
// Each event emits: (topic_symbol, bounty_id) + relevant data in the data slot.
// ---------------------------------------------------------------------------

/// Contract initialized.
pub fn emit_initialized(env: &Env, admin: &Address, fee_bps: u32) {
    env.events().publish(
        (symbol_short!("init"), admin.clone()),
        fee_bps,
    );
}

/// New bounty posted — escrow locked.
pub fn emit_bounty_posted(env: &Env, bounty_id: BountyId, poster: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("posted"), bounty_id),
        (poster.clone(), amount),
    );
}

/// Contributor claimed a bounty.
pub fn emit_bounty_claimed(env: &Env, bounty_id: BountyId, contributor: &Address) {
    env.events().publish(
        (symbol_short!("claimed"), bounty_id),
        contributor.clone(),
    );
}

/// Contributor submitted work.
pub fn emit_work_submitted(env: &Env, bounty_id: BountyId, contributor: &Address, work_hash: &String) {
    env.events().publish(
        (symbol_short!("submitted"), bounty_id),
        (contributor.clone(), work_hash.clone()),
    );
}

/// Poster approved — escrow released.
pub fn emit_approved(env: &Env, bounty_id: BountyId, contributor: &Address, payout: i128) {
    env.events().publish(
        (symbol_short!("approved"), bounty_id),
        (contributor.clone(), payout),
    );
}

/// Poster rejected work — bounty reset to Open.
pub fn emit_rejected(env: &Env, bounty_id: BountyId, poster: &Address) {
    env.events().publish(
        (symbol_short!("rejected"), bounty_id),
        poster.clone(),
    );
}

/// Poster cancelled bounty — escrow refunded.
pub fn emit_cancelled(env: &Env, bounty_id: BountyId, poster: &Address) {
    env.events().publish(
        (symbol_short!("cancelled"), bounty_id),
        poster.clone(),
    );
}

/// Dispute raised by a party.
pub fn emit_disputed(env: &Env, bounty_id: BountyId, caller: &Address) {
    env.events().publish(
        (symbol_short!("disputed"), bounty_id),
        caller.clone(),
    );
}

/// Dispute resolved by admin with a split.
pub fn emit_resolved(
    env: &Env,
    bounty_id: BountyId,
    resolver: &Address,
    contributor_pct: u32,
) {
    env.events().publish(
        (symbol_short!("resolved"), bounty_id),
        (resolver.clone(), contributor_pct),
    );
}
