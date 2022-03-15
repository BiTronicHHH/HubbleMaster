use anchor_lang::prelude::ProgramResult;
use anchor_lang::Context;
use num::FromPrimitive;

use crate::BorrowError;
use crate::GlobalConfigOption;
use crate::UpdateGlobalConfig;

pub fn process(ctx: Context<UpdateGlobalConfig>, key: u16, value: u64) -> ProgramResult {
    let global_config = &mut ctx.accounts.global_config;
    match GlobalConfigOption::from_u16(key) {
        Some(GlobalConfigOption::IsBorrowingAllowed) => {
            global_config.is_borrowing_allowed = value > 0;
            Ok(())
        }
        Some(GlobalConfigOption::BorrowLimitUsdh) => {
            global_config.borrow_limit_usdh = value;
            Ok(())
        }
        None => Err(BorrowError::GlobalConfigKeyError.into()),
    }
}
