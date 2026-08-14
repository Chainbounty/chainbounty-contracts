#![allow(unused)]

use soroban_sdk::{contracttype, symbol_short, Address, String};

/// Unique bounty identifier (auto-incrementing u64).
pub type BountyId = u64;

/// All possible states a bounty can be in.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum BountyStatus {
    /// Bounty is open and accepting claims.
    Open,
    /// A contributor has claimed the bounty and is working.
    Claimed,
    /// Contributor has submitted work, awaiting poster review.
    Submitted,
    /// Poster approved the submission — escrow released, bounty complete.
    Completed,
    /// Poster or contributor has raised a dispute.
    Disputed,
    /// Admin resolved the dispute.
    Resolved,
    /// Poster cancelled the bounty before a claim — refunded.
    Cancelled,
}

/// Core bounty record stored on-chain.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Bounty {
    /// Auto-assigned unique ID.
    pub id: BountyId,
    /// Address of the user who posted the bounty.
    pub poster: Address,
    /// Stellar token contract address used for escrow.
    pub token: Address,
    /// Escrowed reward amount in the token's stroops/smallest unit.
    pub amount: i128,
    /// Short title for the bounty.
    pub title: String,
    /// IPFS CID or content hash of the full description.
    pub description_hash: String,
    /// Unix timestamp deadline for submission.
    pub deadline: u64,
    /// Current lifecycle status.
    pub status: BountyStatus,
    /// Address of the contributor who claimed the bounty (if any).
    pub contributor: Option<Address>,
    /// IPFS hash of the submitted work (if submitted).
    pub work_hash: Option<String>,
    /// Ledger timestamp when the bounty was posted.
    pub created_at: u64,
    /// Ledger timestamp of the last status change.
    pub updated_at: u64,
}

/// Contract-level storage keys.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Contract admin address.
    Admin,
    /// Platform fee in basis points (100 = 1%).
    FeeBps,
    /// Total number of bounties ever created.
    BountyCount,
    /// Individual bounty record by ID.
    Bounty(BountyId),
    /// Contributor reputation score by address.
    Reputation(Address),
}

/// All errors the contract can return.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ContractError {
    /// Contract has already been initialized.
    AlreadyInitialized,
    /// Contract has not been initialized.
    NotInitialized,
    /// Caller is not authorized to perform this action.
    Unauthorized,
    /// The referenced bounty does not exist.
    BountyNotFound,
    /// The bounty is not in the required status for this operation.
    InvalidStatus,
    /// The provided amount is zero or negative.
    InvalidAmount,
    /// The provided deadline is in the past.
    InvalidDeadline,
    /// The fee basis points exceed the maximum (1000 = 10%).
    InvalidFee,
    /// The title or hash string is empty.
    InvalidInput,
    /// The bounty deadline has passed.
    DeadlineExpired,
    /// Dispute split percentage is out of range (must be 0–100).
    InvalidSplit,
    /// Caller is not a party to this bounty.
    NotAParty,
}
