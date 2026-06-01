use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, String,
};

#[derive(Clone)]
#[contracttype]
pub enum GovernanceDataKey {
    Admin, // To manage governance settings
    RequiredApprovals,
    ProposalCount,
    Proposal(u32),
    UserVote(u32, Address), // Proposal ID, User -> bool
    ConfigValue(String),    // Stores the actual config data
}

#[derive(Clone)]
#[contracttype]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub config_key: String,
    pub config_value: String,
    pub approvals: u32,
    pub executed: bool,
    pub deadline: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernanceError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    ProposalNotFound = 4,
    AlreadyVoted = 5,
    ProposalExpired = 6,
    AlreadyExecuted = 7,
    NotEnoughApprovals = 8,
    Overflow = 9,
    InvalidInput = 10,
}

pub struct GovernanceEvents;

impl GovernanceEvents {
    pub fn admin_updated(env: &Env, previous_admin: &Address, new_admin: &Address) {
        let topics = (symbol_short!("gov"), symbol_short!("admin"));
        env.events().publish(
            topics,
            (
                previous_admin.clone(),
                new_admin.clone(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn proposal_created(
        env: &Env,
        id: u32,
        proposer: &Address,
        config_key: &String,
        config_value: &String,
    ) {
        let topics = (symbol_short!("gov"), symbol_short!("created"));
        env.events().publish(
            topics,
            (
                id,
                proposer.clone(),
                config_key.clone(),
                config_value.clone(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn voted(env: &Env, id: u32, voter: &Address) {
        let topics = (symbol_short!("gov"), symbol_short!("voted"));
        env.events()
            .publish(topics, (id, voter.clone(), env.ledger().timestamp()));
    }

    pub fn proposal_executed(env: &Env, id: u32, config_key: &String, config_value: &String) {
        let topics = (symbol_short!("gov"), symbol_short!("executed"));
        env.events().publish(
            topics,
            (
                id,
                config_key.clone(),
                config_value.clone(),
                env.ledger().timestamp(),
            ),
        );
    }
}

pub fn initialize_governance(env: &Env, admin: Address, required_approvals: u32) {
    if env.storage().instance().has(&GovernanceDataKey::Admin) {
        panic_with_error!(env, GovernanceError::AlreadyInitialized);
    }
    env.storage()
        .instance()
        .set(&GovernanceDataKey::Admin, &admin);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::RequiredApprovals, &required_approvals);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::ProposalCount, &0u32);
}

pub fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::NotInitialized));
    if admin != *caller {
        panic_with_error!(env, GovernanceError::Unauthorized);
    }
}

pub fn update_admin(env: &Env, current_admin: Address, new_admin: Address) {
    require_admin(env, &current_admin);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::Admin, &new_admin);
    GovernanceEvents::admin_updated(env, &current_admin, &new_admin);
}

/// Maximum length for config key and value strings
const MAX_CONFIG_STRING_LENGTH: u32 = 256;

pub fn create_proposal(
    env: &Env,
    proposer: Address,
    config_key: String,
    config_value: String,
    duration_seconds: u64,
) -> u32 {
    proposer.require_auth();

    // Validate input string lengths
    if config_key.len() > MAX_CONFIG_STRING_LENGTH || config_key.len() == 0 {
        panic_with_error!(env, GovernanceError::InvalidInput);
    }
    if config_value.len() > MAX_CONFIG_STRING_LENGTH {
        panic_with_error!(env, GovernanceError::InvalidInput);
    }
    if duration_seconds == 0 {
        panic_with_error!(env, GovernanceError::InvalidInput);
    }

    let count: u32 = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::ProposalCount)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::NotInitialized));

    let new_id = count
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::Overflow));
    let current_time = env.ledger().timestamp();
    let deadline = current_time
        .checked_add(duration_seconds)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::Overflow));

    let proposal = Proposal {
        id: new_id,
        proposer: proposer.clone(),
        config_key: config_key.clone(),
        config_value: config_value.clone(),
        approvals: 0,
        executed: false,
        deadline,
    };

    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(new_id), &proposal);
    env.storage()
        .instance()
        .set(&GovernanceDataKey::ProposalCount, &new_id);

    GovernanceEvents::proposal_created(env, new_id, &proposer, &config_key, &config_value);

    new_id
}

pub fn vote_proposal(env: &Env, voter: Address, proposal_id: u32) {
    voter.require_auth();

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::ProposalNotFound));

    if proposal.executed {
        panic_with_error!(env, GovernanceError::AlreadyExecuted);
    }

    if env.ledger().timestamp() > proposal.deadline {
        panic_with_error!(env, GovernanceError::ProposalExpired);
    }

    let vote_key = GovernanceDataKey::UserVote(proposal_id, voter.clone());
    let has_voted: bool = env.storage().persistent().get(&vote_key).unwrap_or(false);

    if has_voted {
        panic_with_error!(env, GovernanceError::AlreadyVoted);
    }

    env.storage().persistent().set(&vote_key, &true);
    proposal.approvals = proposal
        .approvals
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::Overflow));
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    GovernanceEvents::voted(env, proposal_id, &voter);
}

pub fn execute_proposal(env: &Env, caller: Address, proposal_id: u32) {
    caller.require_auth(); // Anyone can trigger execution if conditions met, but auth required to trace

    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&GovernanceDataKey::Proposal(proposal_id))
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::ProposalNotFound));

    if proposal.executed {
        panic_with_error!(env, GovernanceError::AlreadyExecuted);
    }

    if env.ledger().timestamp() > proposal.deadline {
        panic_with_error!(env, GovernanceError::ProposalExpired);
    }

    let required_approvals: u32 = env
        .storage()
        .instance()
        .get(&GovernanceDataKey::RequiredApprovals)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::NotInitialized));

    if proposal.approvals < required_approvals {
        panic_with_error!(env, GovernanceError::NotEnoughApprovals);
    }

    // Apply configuration changes
    env.storage().persistent().set(
        &GovernanceDataKey::ConfigValue(proposal.config_key.clone()),
        &proposal.config_value,
    );

    proposal.executed = true;
    env.storage()
        .persistent()
        .set(&GovernanceDataKey::Proposal(proposal_id), &proposal);

    GovernanceEvents::proposal_executed(
        env,
        proposal_id,
        &proposal.config_key,
        &proposal.config_value,
    );
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    pub fn initialize(env: Env, admin: Address, required_approvals: u32) {
        initialize_governance(&env, admin, required_approvals);
    }

    pub fn update_admin(env: Env, current_admin: Address, new_admin: Address) {
        update_admin(&env, current_admin, new_admin);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&GovernanceDataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, GovernanceError::NotInitialized))
    }

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        config_key: String,
        config_value: String,
        duration_seconds: u64,
    ) -> u32 {
        create_proposal(&env, proposer, config_key, config_value, duration_seconds)
    }

    pub fn vote_proposal(env: Env, voter: Address, proposal_id: u32) {
        vote_proposal(&env, voter, proposal_id);
    }

    pub fn execute_proposal(env: Env, caller: Address, proposal_id: u32) {
        execute_proposal(&env, caller, proposal_id);
    }

    pub fn get_proposal(env: Env, proposal_id: u32) -> Option<Proposal> {
        env.storage()
            .persistent()
            .get(&GovernanceDataKey::Proposal(proposal_id))
    }

    pub fn get_config(env: Env, config_key: String) -> Option<String> {
        env.storage()
            .persistent()
            .get(&GovernanceDataKey::ConfigValue(config_key))
    }
}
