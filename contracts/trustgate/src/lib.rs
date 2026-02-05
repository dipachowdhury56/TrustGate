#![no_std]
pub mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, String};
use types::{DataKey, Error, EscrowAgreement, Milestone, MilestoneStatus};

#[contract]
pub struct TrustGateEscrow;

#[contractimpl]
impl TrustGateEscrow {
    pub fn create_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        arbiter: Address,
        token: Address,
        review_window_seconds: u64,
        title: String,
    ) -> Result<u32, Error> {
        buyer.require_auth();

        let escrow_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowCount)
            .unwrap_or(0);

        let new_id = escrow_count + 1;

        let escrow = EscrowAgreement {
            id: new_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            arbiter: arbiter.clone(),
            token: token.clone(),
            total_amount: 0,
            released_amount: 0,
            review_window_seconds,
            milestones_count: 0,
            is_cancelled: false,
            title,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(new_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::EscrowCount, &new_id);

        env.events().publish(
            (symbol_short!("create"), new_id),
            (buyer, seller, arbiter),
        );

        Ok(new_id)
    }

    pub fn add_milestone(
        env: Env,
        escrow_id: u32,
        amount: i128,
        description: String,
    ) -> Result<u32, Error> {
        let mut escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        escrow.buyer.require_auth();

        if escrow.is_cancelled {
            return Err(Error::AlreadyCancelled);
        }

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(&escrow.buyer, &env.current_contract_address(), &amount);

        let milestone_id = escrow.milestones_count + 1;
        let empty_uri = String::from_str(&env, "");

        let milestone = Milestone {
            id: milestone_id,
            description,
            amount,
            submission_uri: empty_uri,
            submitted_time: 0,
            status: MilestoneStatus::Pending,
        };

        escrow.total_amount += amount;
        escrow.milestones_count = milestone_id;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        env.events().publish(
            (symbol_short!("mile_add"), escrow_id),
            (milestone_id, amount),
        );

        Ok(milestone_id)
    }

    pub fn submit_milestone_work(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
        submission_uri: String,
    ) -> Result<(), Error> {
        let escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        escrow.seller.require_auth();

        if escrow.is_cancelled {
            return Err(Error::AlreadyCancelled);
        }

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Pending {
            return Err(Error::InvalidState);
        }

        milestone.submission_uri = submission_uri;
        milestone.submitted_time = env.ledger().timestamp();
        milestone.status = MilestoneStatus::Submitted;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);

        env.events().publish(
            (symbol_short!("mile_sub"), escrow_id),
            (milestone_id, milestone.submitted_time),
        );

        Ok(())
    }

    pub fn approve_milestone(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
    ) -> Result<(), Error> {
        let mut escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        escrow.buyer.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Submitted
            && milestone.status != MilestoneStatus::Pending
        {
            return Err(Error::InvalidState);
        }

        milestone.status = MilestoneStatus::Approved;
        escrow.released_amount += milestone.amount;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.seller,
            &milestone.amount,
        );

        env.events().publish(
            (symbol_short!("mile_app"), escrow_id),
            (milestone_id, milestone.amount),
        );

        Ok(())
    }

    pub fn claim_auto_release(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
    ) -> Result<(), Error> {
        let mut escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        escrow.seller.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Submitted {
            return Err(Error::InvalidState);
        }

        let current_time = env.ledger().timestamp();
        if current_time < milestone.submitted_time + escrow.review_window_seconds {
            return Err(Error::ReviewPeriodNotElapsed);
        }

        milestone.status = MilestoneStatus::Approved;
        escrow.released_amount += milestone.amount;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.seller,
            &milestone.amount,
        );

        env.events().publish(
            (symbol_short!("mile_auto"), escrow_id),
            (milestone_id, milestone.amount),
        );

        Ok(())
    }

    pub fn dispute_milestone(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
        caller: Address,
    ) -> Result<(), Error> {
        caller.require_auth();

        let escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if caller != escrow.buyer && caller != escrow.seller {
            return Err(Error::Unauthorized);
        }

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Submitted
            && milestone.status != MilestoneStatus::Pending
        {
            return Err(Error::InvalidState);
        }

        milestone.status = MilestoneStatus::Disputed;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);

        env.events().publish(
            (symbol_short!("dispute"), escrow_id),
            (milestone_id, caller),
        );

        Ok(())
    }

    pub fn resolve_dispute(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
        seller_share: i128,
        buyer_share: i128,
    ) -> Result<(), Error> {
        let mut escrow: EscrowAgreement = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        escrow.arbiter.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.status != MilestoneStatus::Disputed {
            return Err(Error::InvalidState);
        }

        if seller_share < 0 || buyer_share < 0 || seller_share + buyer_share != milestone.amount {
            return Err(Error::InvalidSplit);
        }

        milestone.status = MilestoneStatus::Resolved;
        escrow.released_amount += milestone.amount;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(escrow_id, milestone_id), &milestone);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        let token_client = token::Client::new(&env, &escrow.token);
        if seller_share > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.seller,
                &seller_share,
            );
        }
        if buyer_share > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.buyer,
                &buyer_share,
            );
        }

        env.events().publish(
            (symbol_short!("resolved"), escrow_id),
            (milestone_id, seller_share, buyer_share),
        );

        Ok(())
    }

    pub fn get_escrow(env: Env, escrow_id: u32) -> Result<EscrowAgreement, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)
    }

    pub fn get_milestone(
        env: Env,
        escrow_id: u32,
        milestone_id: u32,
    ) -> Result<Milestone, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(escrow_id, milestone_id))
            .ok_or(Error::MilestoneNotFound)
    }
}
