pub const DECIMAL_PRECISION: u128 = 1_000_000_000_000;
pub const ONE: u128 = DECIMAL_PRECISION;
pub const SCALE_FACTOR: u128 = 1_000_000_000;

pub const SRM_DECIMALS: u8 = 6;
pub const RAY_DECIMALS: u8 = 6;
pub const FTT_DECIMALS: u8 = 6;
pub const ETH_DECIMALS: u8 = 6;
pub const BTC_DECIMALS: u8 = 6;
pub const SOL_DECIMALS: u8 = 9;
pub const USDH_DECIMALS: u8 = 6;
pub const USDC_DECIMALS: u8 = 6;
pub const HBB_DECIMALS: u8 = 6;

pub const SRM_PYTH_EXPONENT: u8 = 8;
pub const RAY_PYTH_EXPONENT: u8 = 8;
pub const FTT_PYTH_EXPONENT: u8 = 8;
pub const ETH_PYTH_EXPONENT: u8 = 8;
pub const BTC_PYTH_EXPONENT: u8 = 8;
pub const SOL_PYTH_EXPONENT: u8 = 8;
pub const USDH_PYTH_EXPONENT: u8 = 8;
pub const USDC_PYTH_EXPONENT: u8 = 8;
pub const HBB_PYTH_EXPONENT: u8 = 8;

pub const STABLECOIN_FACTOR: u64 = 1_000_000; // 6 decimals
pub const HBB_FACTOR: u64 = 1_000_000; // 6 decimals

pub const REDEMPTION_STAKERS: u16 = 40; // 0.4%
pub const REDEMPTION_FILLER: u16 = 5; // 0.005%
pub const REDEMPTION_CLEARER: u16 = 5; // 0.005%

pub const LIQUIDATIONS_SECONDS_TO_CLAIM_GAINS: u64 = 5;
pub const MAX_LIQUIDATION_EVENTS: usize = 300;

// can make this bigger and run tests with RUST_MIN_STACK=8388608 cargo test
// but we need to make this a seed-generated address and keep track of index
pub const MAX_REDEMPTION_EVENTS: usize = 15;
pub const REDEMPTIONS_SECONDS_TO_FILL_ORDER: u64 = 5;
pub const MIN_REDEMPTIONS_AMOUNT_USDH: u64 = 2000 * STABLECOIN_FACTOR;

pub const LIQUIDATOR_RATE: u16 = 40; // 0.004 -> 0.4% -> bps
pub const CLEARER_RATE: u16 = 10; // 0.001 -> 0.1% -> bps
pub const NORMAL_MCR: u8 = 110; // percent
pub const RECOVERY_MCR: u8 = 150; // percent

// no point to add further complexity with leap years and seconds for this particular case
pub const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;
pub const SECONDS_PER_MINUTE: u64 = 60;

pub const TOTAL_HBB_SUPPLY: u64 = 100_000_000;
pub const TOTAL_HBB_TO_STABILITY_POOL: u64 = 31_000_000;
pub const HBB_ISSUANCE_FACTOR: u64 = 999998681227695000;

pub const BORROW_MIN: u64 = 200_000_000;

/*
 * Half-life of 12h. 12h = 720 min
 * (1/2) = d^720 => d = (1/2)^(1/720)
 */
pub const MINUTE_DECAY_FACTOR: u64 = 999037758833783000;
pub const REDEMPTION_FEE_FLOOR: u16 = 50; // 50 bps, 0.5%
pub const MAX_REDEMPTION_FEE: u16 = 10000; // 10_000 bps, 100%
pub const MAX_BORROWING_FEE: u16 = 500; // 500 bps, 5%
pub const BORROWING_FEE_FLOOR: u16 = 50; // 50 bps, 0.5%
pub const BOOTSTRAP_PERIOD: u64 = 0; // 14 days
                                     // pub const BOOTSTRAP_PERIOD: u64 = 14 * 24 * 60 * 60; // 14 days

// pub const REDEMPTION_FEE_FLOOR: u64 = DECIMAL_PRECISION / 1000 * 5; // 0.5%
// pub const MAX_BORROWING_FEE: u64 = DECIMAL_PRECISION / 100 * 5; // 5%

// Issuing 58881157 HBB as of 31536000 with an existing 154941118843
// Issuing 29437647 HBB as of 31536000 with an existing 154970562353

// 154941118843 + 58881157 = 155000000000
// 154970562353 + 29437647 = 155000000000

// 154999999930 + 237 = 155000000167
