use anchor_lang::prelude::Pubkey;

#[derive(Clone, Copy, Debug)]
pub enum PDA {
    BorrowingFeesAccount { owner: Pubkey },
    StablecoinMint { owner: Pubkey },
    StabilityPool { owner: Pubkey },
    BurningPotAccount { owner: Pubkey },
    StakingPool { owner: Pubkey },
    HbbMint { owner: Pubkey },
    CollateralVault { owner: Pubkey },
    LiquidationsVault { owner: Pubkey },
}

impl PDA {
    pub fn collateral_vault_from(owner: &Pubkey) -> Self {
        PDA::CollateralVault { owner: *owner }
    }
    pub fn liquidation_rewards_vault_from(owner: &Pubkey) -> Self {
        PDA::LiquidationsVault { owner: *owner }
    }
}

#[derive(Debug)]
pub struct PdaAddress {
    pub key: Pubkey,
    pub seed: u8,
}

pub const BORROWING_FEE_TAG: &str = "bfa";
pub const STABILITY_POOL_TAG: &str = "spa";
pub const STABLECOIN_MINT_TAG: &str = "sma";
pub const BURNING_POT_TAG: &str = "bpa";
pub const STAKING_POOL_TAG: &str = "stpa";
pub const HBB_MINT_TAG: &str = "hma";
pub const COLL_VAULT_TAG: &str = "colv";
pub const LIQ_VAULT_TAG: &str = "liqv";

pub fn make_pda_pubkey(mode: PDA, program: &Pubkey) -> PdaAddress {
    match &mode {
        PDA::BorrowingFeesAccount { owner } => make_pda(owner, BORROWING_FEE_TAG, program),
        PDA::BurningPotAccount { owner } => make_pda(owner, BURNING_POT_TAG, program),
        PDA::StabilityPool { owner } => make_pda(owner, STABILITY_POOL_TAG, program),
        PDA::StablecoinMint { owner } => make_pda(owner, STABLECOIN_MINT_TAG, program),
        PDA::StakingPool { owner } => make_pda(owner, STAKING_POOL_TAG, program),
        PDA::HbbMint { owner } => make_pda(owner, HBB_MINT_TAG, program),
        PDA::CollateralVault { owner } => make_pda(owner, COLL_VAULT_TAG, program),
        PDA::LiquidationsVault { owner } => make_pda(owner, LIQ_VAULT_TAG, program),
    }
}

pub fn make_pda_seeds<'a>(mode: &'a PDA, _program: &'a Pubkey) -> [Vec<u8>; 2] {
    match &mode {
        PDA::BorrowingFeesAccount { owner } => make_seeds(owner, BORROWING_FEE_TAG),
        PDA::BurningPotAccount { owner } => make_seeds(owner, BURNING_POT_TAG),
        PDA::StabilityPool { owner } => make_seeds(owner, STABILITY_POOL_TAG),
        PDA::StablecoinMint { owner } => make_seeds(owner, STABLECOIN_MINT_TAG),
        PDA::StakingPool { owner } => make_seeds(owner, STAKING_POOL_TAG),
        PDA::HbbMint { owner } => make_seeds(owner, HBB_MINT_TAG),
        PDA::CollateralVault { owner } => make_seeds(owner, COLL_VAULT_TAG),
        PDA::LiquidationsVault { owner } => make_seeds(owner, LIQ_VAULT_TAG),
    }
}

fn make_seeds(owner: &Pubkey, tag: &'static str) -> [Vec<u8>; 2] {
    let signer_seeds = [owner.as_ref().to_owned(), tag.as_bytes().to_owned()];
    signer_seeds
}

pub fn make_pda(owner: &Pubkey, tag: &str, program: &Pubkey) -> PdaAddress {
    let seeds = &[owner.as_ref(), tag.as_ref()];
    let (key, seed) = Pubkey::find_program_address(seeds, program);
    PdaAddress { key, seed }
}

// fn drop_reward() {
//     let total_amount_of_coins = 1_000_000;
//     let amount_dropped = 200;

//     let time_last_dropped = 0;
//     let time_current = 100;
//     let time_step = 5;

//     if amount_dropped >= total_amount_of_coins {
//         println!("Already dropped total number of coins");
//     }

//     if time_current < time_last_dropped {
//         println!("Very wrong, we are in the past, time current should be in the future of time_last_dropped");
//     }

//     if (time_current - time_last_dropped) < time_step {
//         println!("Need to wait a bit longer for the next reward to drop");
//     }

//     // let amount_expected_till_now =
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_generate_borrowing_seed() {
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let pdapubkey = make_pda_pubkey(
            PDA::BorrowingFeesAccount {
                owner: owner.clone(),
            },
            &program_id,
        );
        println!("pubkey {:?}", &pdapubkey);
    }
    #[test]
    fn test_generate_global_seed() {
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let pdapubkey = make_pda_pubkey(
            PDA::StabilityPool {
                owner: owner.clone(),
            },
            &program_id,
        );
        println!("pubkey {:?}", &pdapubkey);
    }
    #[test]
    fn test_generate_stable_seed() {
        //8gPfCN6L2JJqML4CJYyQjnuiKJ8ALXSK5jpvYN6XekMQ
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let pdapubkey = make_pda_pubkey(
            PDA::StablecoinMint {
                owner: owner.clone(),
            },
            &program_id,
        );
        println!("pubkey {:?}", &pdapubkey);
    }

    #[test]
    fn test_generate_burning_seed() {
        //8gPfCN6L2JJqML4CJYyQjnuiKJ8ALXSK5jpvYN6XekMQ
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let pdapubkey = make_pda_pubkey(
            PDA::BurningPotAccount {
                owner: owner.clone(),
            },
            &program_id,
        );
        println!("pubkey {:?}", &pdapubkey);
    }

    #[test]
    fn test_generate_stabliblity_seed() {
        //8gPfCN6L2JJqML4CJYyQjnuiKJ8ALXSK5jpvYN6XekMQ
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let pdapubkey = make_pda_pubkey(
            PDA::StabilityPool {
                owner: owner.clone(),
            },
            &program_id,
        );
        println!("pubkey {:?}", &pdapubkey);
    }

    #[test]
    fn test_create_borrow_account() {
        // Expecting GBoLHBwYmHQfGBQcHMoW5icybKNQzudWQTzn1UsfJbiK
        let owner = Pubkey::from_str("BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y").unwrap();
        let program_id = Pubkey::from_str("7SeC6f66GuxEEE1PHmAabu1SYbLnayJkWNE6127BNUYc").unwrap();
        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"borrowing_fees2"], &program_id);
        let x = Pubkey::create_with_seed(&owner, "borrowing_fees2", &program_id);
        println!("key {:?}", pda);
        println!("x {:?}", x);
    }
}
