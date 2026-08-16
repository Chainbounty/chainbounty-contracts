use soroban_sdk::{Env, String};

use crate::types::ContractError;

/// Maximum allowed platform fee: 10% in basis points.
pub const MAX_FEE_BPS: u32 = 1000;

/// Minimum title length (characters).
pub const MIN_TITLE_LEN: u32 = 3;

/// Validate platform fee is within acceptable range.
pub fn require_valid_fee(fee_bps: u32) -> Result<(), ContractError> {
    if fee_bps > MAX_FEE_BPS {
        return Err(ContractError::InvalidFee);
    }
    Ok(())
}

/// Validate amount is positive.
pub fn require_positive_amount(amount: i128) -> Result<(), ContractError> {
    if amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }
    Ok(())
}

/// Validate deadline is in the future.
pub fn require_future_deadline(env: &Env, deadline: u64) -> Result<(), ContractError> {
    let now = env.ledger().timestamp();
    if deadline <= now {
        return Err(ContractError::InvalidDeadline);
    }
    Ok(())
}

/// Validate deadline has not yet passed (used at claim/submit time).
pub fn require_not_expired(env: &Env, deadline: u64) -> Result<(), ContractError> {
    let now = env.ledger().timestamp();
    if now > deadline {
        return Err(ContractError::DeadlineExpired);
    }
    Ok(())
}

/// Validate a string input is non-empty and meets minimum length.
pub fn require_non_empty_string(s: &String) -> Result<(), ContractError> {
    if s.len() < MIN_TITLE_LEN {
        return Err(ContractError::InvalidInput);
    }
    Ok(())
}

/// Validate dispute split percentage is between 0 and 100.
pub fn require_valid_split(pct: u32) -> Result<(), ContractError> {
    if pct > 100 {
        return Err(ContractError::InvalidSplit);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Fee helpers (shared across bounty and dispute modules)
// ---------------------------------------------------------------------------

/// Read the platform fee basis points from contract instance storage.
/// Returns 0 if not set (safe default — no fee).
pub fn get_fee_bps(env: &Env) -> u32 {
    use crate::types::DataKey;
    env.storage()
        .instance()
        .get(&DataKey::FeeBps)
        .unwrap_or(0u32)
}

/// Compute `(net_payout, platform_fee)` from a gross amount and fee basis points.
///
/// Fee = amount × fee_bps / 10_000  (integer division, truncates in favour of payee)
/// Net  = amount − fee
pub fn compute_fee_split(amount: i128, fee_bps: u32) -> (i128, i128) {
    let fee = amount * fee_bps as i128 / 10_000;
    let net = amount - fee;
    (net, fee)
}
