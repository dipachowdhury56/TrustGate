#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(env, &contract_address.address()),
        token::StellarAssetClient::new(env, &contract_address.address()),
    )
}

#[test]
fn test_escrow_multi_milestone_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let contract_id = env.register(TrustGateEscrow, ());
    let client = TrustGateEscrowClient::new(&env, &contract_id);

    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&buyer, &2000);

    let title = String::from_str(&env, "Fullstack Web3 App Development");
    let review_window = 500u64;

    let escrow_id = client.create_escrow(
        &buyer,
        &seller,
        &arbiter,
        &token_client.address,
        &review_window,
        &title,
    );
    assert_eq!(escrow_id, 1);

    // Add Milestone 1 (400 tokens)
    let m1_desc = String::from_str(&env, "Smart Contract & Testnet Deployment");
    let m1_id = client.add_milestone(&escrow_id, &400, &m1_desc);
    assert_eq!(m1_id, 1);
    assert_eq!(token_client.balance(&contract_id), 400);

    // Add Milestone 2 (600 tokens)
    let m2_desc = String::from_str(&env, "Frontend Integration & QA");
    let m2_id = client.add_milestone(&escrow_id, &600, &m2_desc);
    assert_eq!(m2_id, 2);
    assert_eq!(token_client.balance(&contract_id), 1000);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.total_amount, 1000);
    assert_eq!(escrow.milestones_count, 2);

    // Seller submits work for Milestone 1
    let m1_proof = String::from_str(&env, "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");
    client.submit_milestone_work(&escrow_id, &m1_id, &m1_proof);

    let m1 = client.get_milestone(&escrow_id, &m1_id);
    assert_eq!(m1.status, MilestoneStatus::Submitted);

    // Buyer approves Milestone 1
    client.approve_milestone(&escrow_id, &m1_id);
    assert_eq!(token_client.balance(&seller), 400);
    assert_eq!(token_client.balance(&contract_id), 600);

    // Seller submits work for Milestone 2
    let m2_proof = String::from_str(&env, "ipfs://bafybeihdwdcefgh463f27hdt3t4vghj2364724");
    client.submit_milestone_work(&escrow_id, &m2_id, &m2_proof);

    // Buyer approves Milestone 2
    client.approve_milestone(&escrow_id, &m2_id);
    assert_eq!(token_client.balance(&seller), 1000);
    assert_eq!(token_client.balance(&contract_id), 0);

    let updated_escrow = client.get_escrow(&escrow_id);
    assert_eq!(updated_escrow.released_amount, 1000);
}

#[test]
fn test_auto_release_after_timeout() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let contract_id = env.register(TrustGateEscrow, ());
    let client = TrustGateEscrowClient::new(&env, &contract_id);

    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&buyer, &1000);

    let title = String::from_str(&env, "Design Systems Sprint");
    let review_window = 300u64; // 300 seconds

    let escrow_id = client.create_escrow(
        &buyer,
        &seller,
        &arbiter,
        &token_client.address,
        &review_window,
        &title,
    );

    let m1_desc = String::from_str(&env, "Figma Wireframes");
    let m1_id = client.add_milestone(&escrow_id, &500, &m1_desc);

    // Seller submits work at t=1000
    let m1_proof = String::from_str(&env, "ipfs://figma-delivery-proof-hash");
    client.submit_milestone_work(&escrow_id, &m1_id, &m1_proof);

    // Advance 200s (t=1200) -> before review window (300s). Claim should fail.
    env.ledger().with_mut(|li| {
        li.timestamp = 1200;
    });
    let res = client.try_claim_auto_release(&escrow_id, &m1_id);
    assert!(res.is_err());

    // Advance past review window to t=1350 (350s after submit)
    env.ledger().with_mut(|li| {
        li.timestamp = 1350;
    });
    client.claim_auto_release(&escrow_id, &m1_id);

    // Seller receives auto-released funds!
    assert_eq!(token_client.balance(&seller), 500);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_dispute_and_arbiter_split_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let contract_id = env.register(TrustGateEscrow, ());
    let client = TrustGateEscrowClient::new(&env, &contract_id);

    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&buyer, &1000);

    let title = String::from_str(&env, "Backend Microservice");
    let escrow_id = client.create_escrow(
        &buyer,
        &seller,
        &arbiter,
        &token_client.address,
        &500,
        &title,
    );

    let m_desc = String::from_str(&env, "Auth Service Implementation");
    let m_id = client.add_milestone(&escrow_id, &1000, &m_desc);

    // Seller submits deliverable
    let m_proof = String::from_str(&env, "ipfs://pr-and-dockerfile");
    client.submit_milestone_work(&escrow_id, &m_id, &m_proof);

    // Buyer is unhappy with code quality and files dispute
    client.dispute_milestone(&escrow_id, &m_id, &buyer);
    let m = client.get_milestone(&escrow_id, &m_id);
    assert_eq!(m.status, MilestoneStatus::Disputed);

    // Arbiter reviews code and rules:
    // Seller gets 700 tokens for partial completion, Buyer refunded 300 tokens
    client.resolve_dispute(&escrow_id, &m_id, &700, &300);

    assert_eq!(token_client.balance(&seller), 700);
    assert_eq!(token_client.balance(&buyer), 300);
    assert_eq!(token_client.balance(&contract_id), 0);

    let resolved_m = client.get_milestone(&escrow_id, &m_id);
    assert_eq!(resolved_m.status, MilestoneStatus::Resolved);
}
