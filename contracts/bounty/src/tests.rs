#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

use crate::{types::ContractError, ChainBountyContract, ChainBountyContractClient};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Setup a test environment with the contract deployed, initialized, and a mock token.
fn setup_test() -> (Env, ChainBountyContractClient, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths(); // All address auth checks pass automatically

    let contract_id = env.register_contract(None, ChainBountyContract);
    let client = ChainBountyContractClient::new(&env, &contract_id);

    // Deploy a test token
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());

    // Create test addresses
    let admin = Address::generate(&env);
    let poster = Address::generate(&env);

    // Initialize contract with 5% platform fee (500 bps)
    client.initialize(&admin, &500);

    // Mint tokens to poster
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&poster, &1_000_000);

    (env, client, admin, poster, token_id)
}

// ---------------------------------------------------------------------------
// Post bounty tests
// ---------------------------------------------------------------------------

#[test]
fn test_post_bounty_success() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Fix critical bug");
    let desc = String::from_str(&env, "QmXyz123456789");
    let amount = 100_000i128;
    let deadline = env.ledger().timestamp() + 86400; // 1 day from now

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    assert_eq!(bounty_id, 1);

    // Verify bounty state
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.id, 1);
    assert_eq!(bounty.poster, poster);
    assert_eq!(bounty.amount, amount);
    assert_eq!(bounty.title, title);
}

#[test]
fn test_post_bounty_invalid_amount() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let result = client.try_post_bounty(&poster, &token_id, &0, &title, &desc, &deadline);

    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_post_bounty_invalid_deadline() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let past_deadline = env.ledger().timestamp() - 1;

    let result = client.try_post_bounty(&poster, &token_id, &1000, &title, &desc, &past_deadline);

    assert_eq!(result, Err(Ok(ContractError::InvalidDeadline)));
}

#[test]
fn test_post_bounty_invalid_title() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let short_title = String::from_str(&env, "ab"); // too short (< 3 chars)
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let result = client.try_post_bounty(&poster, &token_id, &1000, &short_title, &desc, &deadline);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

// ---------------------------------------------------------------------------
// Claim bounty tests
// ---------------------------------------------------------------------------

#[test]
fn test_claim_bounty_success() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Fix bug");
    let desc = String::from_str(&env, "QmAbc");
    let amount = 50_000i128;
    let deadline = env.ledger().timestamp() + 86400;

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    // Verify claim
    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.contributor, Some(contributor));
    assert_eq!(bounty.status, crate::types::BountyStatus::Claimed);
}

#[test]
fn test_claim_bounty_already_claimed() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor1 = Address::generate(&env);
    client.claim_bounty(&contributor1, &bounty_id);

    // Try to claim again with a different contributor
    let contributor2 = Address::generate(&env);
    let result = client.try_claim_bounty(&contributor2, &bounty_id);

    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_claim_bounty_poster_cannot_claim() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    // Poster tries to claim their own bounty
    let result = client.try_claim_bounty(&poster, &bounty_id);

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_claim_bounty_expired() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 100;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    // Advance ledger past deadline
    env.ledger().with_mut(|li| {
        li.timestamp = deadline + 1;
    });

    let contributor = Address::generate(&env);
    let result = client.try_claim_bounty(&contributor, &bounty_id);

    assert_eq!(result, Err(Ok(ContractError::DeadlineExpired)));
}

// ---------------------------------------------------------------------------
// Submit work tests
// ---------------------------------------------------------------------------

#[test]
fn test_submit_work_success() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Fix bug");
    let desc = String::from_str(&env, "QmAbc");
    let deadline = env.ledger().timestamp() + 86400;

    let bounty_id = client.post_bounty(&poster, &token_id, &50000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork123");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.work_hash, Some(work_hash));
    assert_eq!(bounty.status, crate::types::BountyStatus::Submitted);
}

#[test]
fn test_submit_work_not_claimed() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    let work_hash = String::from_str(&env, "QmWork");

    let result = client.try_submit_work(&contributor, &bounty_id, &work_hash);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_submit_work_wrong_contributor() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor1 = Address::generate(&env);
    client.claim_bounty(&contributor1, &bounty_id);

    let contributor2 = Address::generate(&env);
    let work_hash = String::from_str(&env, "QmWork");

    let result = client.try_submit_work(&contributor2, &bounty_id, &work_hash);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Approve submission tests
// ---------------------------------------------------------------------------

#[test]
fn test_approve_submission_success() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;
    let amount = 100_000i128;

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Approve
    client.approve_submission(&poster, &bounty_id);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, crate::types::BountyStatus::Completed);

    // Verify contributor received payout (amount - 5% fee)
    let token_client = token::Client::new(&env, &token_id);
    let contributor_balance = token_client.balance(&contributor);
    // 100_000 * 0.95 = 95_000
    assert_eq!(contributor_balance, 95_000);
}

#[test]
fn test_approve_submission_invalid_status() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    // Try to approve without submission
    let result = client.try_approve_submission(&poster, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_approve_submission_not_poster() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Someone else tries to approve
    let random_user = Address::generate(&env);
    let result = client.try_approve_submission(&random_user, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Reject submission tests
// ---------------------------------------------------------------------------

#[test]
fn test_reject_submission_success() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Reject
    client.reject_submission(&poster, &bounty_id);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, crate::types::BountyStatus::Open);
    assert_eq!(bounty.contributor, None);
    assert_eq!(bounty.work_hash, None);
}

#[test]
fn test_reject_submission_invalid_status() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    // Try to reject Open bounty (no submission yet)
    let result = client.try_reject_submission(&poster, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_reject_submission_not_poster() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Random user tries to reject
    let random_user = Address::generate(&env);
    let result = client.try_reject_submission(&random_user, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Dispute bounty tests
// ---------------------------------------------------------------------------

#[test]
fn test_dispute_bounty_by_poster() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Poster raises dispute
    client.dispute_bounty(&poster, &bounty_id);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, crate::types::BountyStatus::Disputed);
}

#[test]
fn test_dispute_bounty_by_contributor() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Contributor raises dispute
    client.dispute_bounty(&contributor, &bounty_id);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, crate::types::BountyStatus::Disputed);
}

#[test]
fn test_dispute_bounty_invalid_status() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    // Try to dispute an Open bounty (no submission yet)
    let result = client.try_dispute_bounty(&poster, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_dispute_bounty_not_a_party() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Random user tries to dispute
    let random_user = Address::generate(&env);
    let result = client.try_dispute_bounty(&random_user, &bounty_id);
    assert_eq!(result, Err(Ok(ContractError::NotAParty)));
}

// ---------------------------------------------------------------------------
// Resolve dispute tests
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_dispute_50_50_split() {
    let (env, client, admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;
    let amount = 100_000i128;

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    client.dispute_bounty(&poster, &bounty_id);

    // Admin resolves with 50% split
    client.resolve_dispute(&admin, &bounty_id, &50);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.status, crate::types::BountyStatus::Resolved);

    let token_client = token::Client::new(&env, &token_id);

    // Contributor gets 50% of 100k = 50k, minus 5% fee = 47,500
    let contributor_balance = token_client.balance(&contributor);
    assert_eq!(contributor_balance, 47_500);

    // Poster gets 50% refund = 50,000
    let poster_balance = token_client.balance(&poster);
    // Poster started with 1M, spent 100k on bounty, got 50k back = 950,000
    assert_eq!(poster_balance, 950_000);

    // Admin gets the 5% fee on contributor's 50k = 2,500
    let admin_balance = token_client.balance(&admin);
    assert_eq!(admin_balance, 2_500);
}

#[test]
fn test_resolve_dispute_100_to_contributor() {
    let (env, client, admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;
    let amount = 100_000i128;

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    client.dispute_bounty(&contributor, &bounty_id);

    // Admin awards 100% to contributor
    client.resolve_dispute(&admin, &bounty_id, &100);

    let token_client = token::Client::new(&env, &token_id);

    // Contributor gets 100k minus 5% fee = 95,000
    let contributor_balance = token_client.balance(&contributor);
    assert_eq!(contributor_balance, 95_000);

    // Poster gets 0 refund
    let poster_balance = token_client.balance(&poster);
    assert_eq!(poster_balance, 900_000); // started with 1M, spent 100k

    // Admin gets 5k fee
    let admin_balance = token_client.balance(&admin);
    assert_eq!(admin_balance, 5_000);
}

#[test]
fn test_resolve_dispute_0_to_contributor() {
    let (env, client, admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;
    let amount = 100_000i128;

    let bounty_id = client.post_bounty(&poster, &token_id, &amount, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    client.dispute_bounty(&poster, &bounty_id);

    // Admin awards 0% to contributor (full refund to poster)
    client.resolve_dispute(&admin, &bounty_id, &0);

    let token_client = token::Client::new(&env, &token_id);

    // Contributor gets 0
    let contributor_balance = token_client.balance(&contributor);
    assert_eq!(contributor_balance, 0);

    // Poster gets full 100k refund
    let poster_balance = token_client.balance(&poster);
    assert_eq!(poster_balance, 1_000_000); // 1M - 100k + 100k = 1M

    // Admin gets no fee (0% to contributor means no fee deduction)
    let admin_balance = token_client.balance(&admin);
    assert_eq!(admin_balance, 0);
}

#[test]
fn test_resolve_dispute_not_admin() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    client.dispute_bounty(&poster, &bounty_id);

    // Random user tries to resolve
    let random_user = Address::generate(&env);
    let result = client.try_resolve_dispute(&random_user, &bounty_id, &50);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_resolve_dispute_invalid_split() {
    let (env, client, admin, poster, token_id) = setup_test();

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);

    let contributor = Address::generate(&env);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    client.dispute_bounty(&poster, &bounty_id);

    // Try to resolve with invalid percentage > 100
    let result = client.try_resolve_dispute(&admin, &bounty_id, &101);
    assert_eq!(result, Err(Ok(ContractError::InvalidSplit)));
}
