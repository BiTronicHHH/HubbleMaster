use anchor_lang::prelude::{msg, AccountInfo, ProgramResult, Pubkey};

use crate::{pda, token_operations::spltoken};

#[allow(clippy::too_many_arguments)]
pub fn mint<'info>(
    amount: u64,
    hbb_mint_seed: u8,
    owner: Pubkey,
    program_id: &Pubkey,
    hbb_mint: AccountInfo<'info>,
    mint_to: AccountInfo<'info>,
    hbb_mint_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
) -> ProgramResult {
    let pda_mode = pda::PDA::HbbMint { owner };
    crate::token_operations::spltoken::mint(
        &hbb_mint,
        &mint_to,
        &hbb_mint_authority,
        hbb_mint_seed,
        pda_mode,
        &token_program,
        program_id,
        amount,
    )
}

pub fn transfer<'info>(
    amount: u64,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> ProgramResult {
    crate::token_operations::spltoken::transfer_from_user(
        amount,
        from,
        to,
        authority,
        token_program,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_staking_pool<'info>(
    amount: u64,
    owner: Pubkey,
    to_account: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    from_vault_authority_seed: u8,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> ProgramResult {
    msg!(
        "Transferring {} HBB from {} to {}",
        amount.to_string(),
        from_vault.clone().key.to_string(),
        to_account.clone().key.to_string(),
    );

    let mode = pda::PDA::StakingPool { owner };
    spltoken::transfer_from_vault(
        amount,
        mode,
        to_account,
        from_vault,
        from_vault_authority,
        from_vault_authority_seed,
        token_program,
        program_id,
    )
}
