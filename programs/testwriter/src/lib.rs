#![allow(clippy::unused_io_amount)]
//! writer program
//!
//! Utility for writing arbitrary data to accounts.
//! Primarily useful for testing, when mocking account data
//! that would normally be set by some other program/process.

use anchor_lang::prelude::*;
use std::io::Write as IoWrite;

declare_id!("AowEgPNmULZQMNQNyWiKWLRN4XogcXqr4VkvpJ69Z5GF");

#[program]
pub mod testwriter {
    use super::*;

    /// Write data to an account
    pub fn write(ctx: Context<Write>, offset: u64, data: Vec<u8>) -> ProgramResult {
        msg!("Writing");
        let account_data = ctx.accounts.target.to_account_info().data;
        let borrow_data = &mut *account_data.borrow_mut();
        let offset = offset as usize;

        (&mut borrow_data[offset..]).write(&data[..])?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Write<'info> {
    #[account(mut, signer)]
    target: AccountInfo<'info>,
}
