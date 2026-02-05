use soroban_sdk::{contracterror, contracttype, Address, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    EscrowNotFound = 1,
    MilestoneNotFound = 2,
    InvalidAmount = 3,
    Unauthorized = 4,
    InvalidState = 5,
    ReviewPeriodNotElapsed = 6,
    InvalidSplit = 7,
    AlreadyCancelled = 8,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MilestoneStatus {
    Pending = 0,
    Submitted = 1,
    Approved = 2,
    Disputed = 3,
    Resolved = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub amount: i128,
    pub submission_uri: String,
    pub submitted_time: u64,
    pub status: MilestoneStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowAgreement {
    pub id: u32,
    pub buyer: Address,
    pub seller: Address,
    pub arbiter: Address,
    pub token: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub review_window_seconds: u64,
    pub milestones_count: u32,
    pub is_cancelled: bool,
    pub title: String,
}

#[contracttype]
pub enum DataKey {
    EscrowCount,
    Escrow(u32),
    Milestone(u32, u32), // (escrow_id, milestone_id)
}
