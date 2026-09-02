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

// ---------------------------------------------------------------------------
// Multi-token support tests
// ---------------------------------------------------------------------------

#[test]
fn test_multi_token_support() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ChainBountyContract);
    let client = ChainBountyContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &500);

    // Create two different tokens
    let token_admin = Address::generate(&env);
    let token_a_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_b_id = env.register_stellar_asset_contract(token_admin.clone());

    let poster1 = Address::generate(&env);
    let poster2 = Address::generate(&env);

    // Mint different tokens to different posters
    let token_a_client = token::StellarAssetClient::new(&env, &token_a_id);
    token_a_client.mint(&poster1, &500_000);

    let token_b_client = token::StellarAssetClient::new(&env, &token_b_id);
    token_b_client.mint(&poster2, &300_000);

    // Post bounty with token A
    let title_a = String::from_str(&env, "Bounty A");
    let desc_a = String::from_str(&env, "QmA");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_a_id = client.post_bounty(&poster1, &token_a_id, &100_000, &title_a, &desc_a, &deadline);

    // Post bounty with token B
    let title_b = String::from_str(&env, "Bounty B");
    let desc_b = String::from_str(&env, "QmB");

    let bounty_b_id = client.post_bounty(&poster2, &token_b_id, &50_000, &title_b, &desc_b, &deadline);

    // Verify both bounties store their respective tokens
    let bounty_a = client.get_bounty(&bounty_a_id);
    assert_eq!(bounty_a.token, token_a_id);
    assert_eq!(bounty_a.amount, 100_000);

    let bounty_b = client.get_bounty(&bounty_b_id);
    assert_eq!(bounty_b.token, token_b_id);
    assert_eq!(bounty_b.amount, 50_000);

    // Claim and complete bounty A with token A payout
    let contributor_a = Address::generate(&env);
    client.claim_bounty(&contributor_a, &bounty_a_id);

    let work_hash_a = String::from_str(&env, "QmWorkA");
    client.submit_work(&contributor_a, &bounty_a_id, &work_hash_a);

    client.approve_submission(&poster1, &bounty_a_id);

    // Verify contributor received token A
    let token_a_balance = token_a_client.balance(&contributor_a);
    assert_eq!(token_a_balance, 95_000); // 100k - 5% fee

    // Claim and complete bounty B with token B payout
    let contributor_b = Address::generate(&env);
    client.claim_bounty(&contributor_b, &bounty_b_id);

    let work_hash_b = String::from_str(&env, "QmWorkB");
    client.submit_work(&contributor_b, &bounty_b_id, &work_hash_b);

    client.approve_submission(&poster2, &bounty_b_id);

    // Verify contributor received token B
    let token_b_balance = token_b_client.balance(&contributor_b);
    assert_eq!(token_b_balance, 47_500); // 50k - 5% fee
}

// ---------------------------------------------------------------------------
// Reputation tests
// ---------------------------------------------------------------------------

#[test]
fn test_reputation_increments_on_approval() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let contributor = Address::generate(&env);

    // Initial reputation should be 0
    let rep_initial = client.get_reputation(&contributor);
    assert_eq!(rep_initial, 0);

    // Complete first bounty
    let title1 = String::from_str(&env, "Bounty 1");
    let desc1 = String::from_str(&env, "QmDesc1");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id1 = client.post_bounty(&poster, &token_id, &10000, &title1, &desc1, &deadline);
    client.claim_bounty(&contributor, &bounty_id1);

    let work_hash1 = String::from_str(&env, "QmWork1");
    client.submit_work(&contributor, &bounty_id1, &work_hash1);
    client.approve_submission(&poster, &bounty_id1);

    // Reputation should be 1
    let rep_after_first = client.get_reputation(&contributor);
    assert_eq!(rep_after_first, 1);

    // Complete second bounty
    let title2 = String::from_str(&env, "Bounty 2");
    let desc2 = String::from_str(&env, "QmDesc2");

    let bounty_id2 = client.post_bounty(&poster, &token_id, &20000, &title2, &desc2, &deadline);
    client.claim_bounty(&contributor, &bounty_id2);

    let work_hash2 = String::from_str(&env, "QmWork2");
    client.submit_work(&contributor, &bounty_id2, &work_hash2);
    client.approve_submission(&poster, &bounty_id2);

    // Reputation should be 2
    let rep_after_second = client.get_reputation(&contributor);
    assert_eq!(rep_after_second, 2);
}

#[test]
fn test_reputation_not_incremented_on_reject() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let contributor = Address::generate(&env);

    let title = String::from_str(&env, "Test");
    let desc = String::from_str(&env, "QmTest");
    let deadline = env.ledger().timestamp() + 1000;

    let bounty_id = client.post_bounty(&poster, &token_id, &10000, &title, &desc, &deadline);
    client.claim_bounty(&contributor, &bounty_id);

    let work_hash = String::from_str(&env, "QmWork");
    client.submit_work(&contributor, &bounty_id, &work_hash);

    // Reject instead of approve
    client.reject_submission(&poster, &bounty_id);

    // Reputation should remain 0
    let rep = client.get_reputation(&contributor);
    assert_eq!(rep, 0);
}

#[test]
fn test_reputation_different_contributors() {
    let (env, client, _admin, poster, token_id) = setup_test();

    let contributor_a = Address::generate(&env);
    let contributor_b = Address::generate(&env);

    let deadline = env.ledger().timestamp() + 1000;

    // Contributor A completes 1 bounty
    let title_a = String::from_str(&env, "Bounty A");
    let desc_a = String::from_str(&env, "QmA");
    let bounty_id_a = client.post_bounty(&poster, &token_id, &10000, &title_a, &desc_a, &deadline);
    client.claim_bounty(&contributor_a, &bounty_id_a);
    let work_a = String::from_str(&env, "QmWorkA");
    client.submit_work(&contributor_a, &bounty_id_a, &work_a);
    client.approve_submission(&poster, &bounty_id_a);

    // Contributor B completes 2 bounties
    let title_b1 = String::from_str(&env, "Bounty B1");
    let desc_b1 = String::from_str(&env, "QmB1");
    let bounty_id_b1 = client.post_bounty(&poster, &token_id, &10000, &title_b1, &desc_b1, &deadline);
    client.claim_bounty(&contributor_b, &bounty_id_b1);
    let work_b1 = String::from_str(&env, "QmWorkB1");
    client.submit_work(&contributor_b, &bounty_id_b1, &work_b1);
    client.approve_submission(&poster, &bounty_id_b1);

    let title_b2 = String::from_str(&env, "Bounty B2");
    let desc_b2 = String::from_str(&env, "QmB2");
    let bounty_id_b2 = client.post_bounty(&poster, &token_id, &10000, &title_b2, &desc_b2, &deadline);
    client.claim_bounty(&contributor_b, &bounty_id_b2);
    let work_b2 = String::from_str(&env, "QmWorkB2");
    client.submit_work(&contributor_b, &bounty_id_b2, &work_b2);
    client.approve_submission(&poster, &bounty_id_b2);

    // Verify individual reputation scores
    assert_eq!(client.get_reputation(&contributor_a), 1);
    assert_eq!(client.get_reputation(&contributor_b), 2);
}
