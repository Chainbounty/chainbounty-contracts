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
