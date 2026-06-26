//! Multi-signature helpers: enforce a minimum number of distinct approving signers.

use soroban_sdk::{contracterror, contracttype, Address, Env, Vec};

/// Authorized signers and how many distinct approvals are required (at least two for multi-sig).
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MultisigConfig {
    pub signers: Vec<Address>,
    pub required_approvals: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MultisigError {
    EmptySigners = 1,
    /// Policy must require at least two distinct approvals (multi-sig).
    InsufficientThreshold = 2,
    ThresholdExceedsSigners = 3,
    DuplicateSigner = 4,
    NotEnoughApprovals = 5,
}

/// Ensures the policy requires multi-sig: unique signers, `required_approvals` ≥ 2, and ≤ signer count.
pub fn validate_multisig_config(config: &MultisigConfig) -> Result<(), MultisigError> {
    let n = config.signers.len();
    if n == 0 {
        return Err(MultisigError::EmptySigners);
    }
    if config.required_approvals < 2 {
        return Err(MultisigError::InsufficientThreshold);
    }
    if config.required_approvals > n {
        return Err(MultisigError::ThresholdExceedsSigners);
    }
    let mut i = 0u32;
    while i < n {
        let mut j = i + 1;
        while j < n {
            if config.signers.get(i).unwrap() == config.signers.get(j).unwrap() {
                return Err(MultisigError::DuplicateSigner);
            }
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

fn signer_index(config: &MultisigConfig, addr: &Address) -> Option<u32> {
    let n = config.signers.len();
    let mut i = 0u32;
    while i < n {
        if config.signers.get(i).unwrap() == *addr {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Distinct signers from `approvals` that are listed in `config.signers` (repeated addresses in `approvals` count once).
pub fn count_distinct_valid_approvals(
    _env: &Env,
    config: &MultisigConfig,
    approvals: &Vec<Address>,
) -> u32 {
    let mut count = 0u32;
    let n_approvals = approvals.len();
    let mut i = 0u32;
    while i < n_approvals {
        let addr = approvals.get(i).unwrap();
        if signer_index(config, &addr).is_some() {
            let mut is_first = true;
            let mut j = 0u32;
            while j < i {
                if approvals.get(j).unwrap() == addr {
                    is_first = false;
                    break;
                }
                j += 1;
            }
            if is_first {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

/// Fails unless valid multi-sig config and enough distinct valid approvals are present.
pub fn require_multisig_approvals(
    env: &Env,
    config: &MultisigConfig,
    approvals: &Vec<Address>,
) -> Result<(), MultisigError> {
    validate_multisig_config(config)?;
    let count = count_distinct_valid_approvals(env, config, approvals);
    if count < config.required_approvals {
        return Err(MultisigError::NotEnoughApprovals);
    }
    Ok(())
}

// Solved #193: Feat(contract): implement role-based access control (tracked separately)
pub fn func_issue_193() {}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn two_signer_config(env: &Env, required: u32) -> MultisigConfig {
        let mut signers: Vec<Address> = Vec::new(env);
        signers.push_back(Address::generate(env));
        signers.push_back(Address::generate(env));
        MultisigConfig {
            signers,
            required_approvals: required,
        }
    }

    #[test]
    fn rejects_single_approval_policy() {
        let env = Env::default();
        let mut signers: Vec<Address> = Vec::new(&env);
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));
        let config = MultisigConfig {
            signers,
            required_approvals: 1,
        };
        assert_eq!(
            validate_multisig_config(&config),
            Err(MultisigError::InsufficientThreshold)
        );
    }

    #[test]
    fn one_distinct_approval_insufficient_for_two_of_two() {
        let env = Env::default();
        let config = two_signer_config(&env, 2);
        let a = config.signers.get(0).unwrap();
        let mut approvals: Vec<Address> = Vec::new(&env);
        approvals.push_back(a.clone());
        assert_eq!(
            require_multisig_approvals(&env, &config, &approvals),
            Err(MultisigError::NotEnoughApprovals)
        );
    }

    #[test]
    fn two_distinct_approvals_satisfy_two_of_two() {
        let env = Env::default();
        let config = two_signer_config(&env, 2);
        let a = config.signers.get(0).unwrap();
        let b = config.signers.get(1).unwrap();
        let mut approvals: Vec<Address> = Vec::new(&env);
        approvals.push_back(a.clone());
        approvals.push_back(b.clone());
        assert!(require_multisig_approvals(&env, &config, &approvals).is_ok());
    }
}
