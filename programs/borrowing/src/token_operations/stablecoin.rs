use anchor_lang::prelude::{msg, AccountInfo, ProgramResult, Pubkey};

use crate::{pda, token_operations::spltoken};

#[allow(clippy::too_many_arguments)]
pub fn mint<'info>(
    amount: u64,
    stablecoin_mint_seed: u8,
    owner: Pubkey,
    program_id: &Pubkey,
    stablecoin_mint: AccountInfo<'info>,
    mint_to: AccountInfo<'info>,
    stablecoin_mint_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
) -> ProgramResult {
    let pda_mode = pda::PDA::StablecoinMint { owner };
    crate::token_operations::spltoken::mint(
        &stablecoin_mint,
        &mint_to,
        &stablecoin_mint_authority,
        stablecoin_mint_seed,
        pda_mode,
        &token_program,
        program_id,
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn burn<'info>(
    amount: u64,
    burn_from: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    burn_authority: &AccountInfo<'info>,
    burn_authority_seed: u8,
    pda_mode: crate::pda::PDA,
    program_id: &Pubkey,
    token_program: &AccountInfo<'info>,
) -> ProgramResult {
    crate::token_operations::spltoken::burn(
        mint,
        burn_from,
        burn_authority,
        burn_authority_seed,
        pda_mode,
        token_program,
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
    spltoken::transfer_from_user(amount, from, to, authority, token_program)
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_stability_pool<'info>(
    amount: u64,
    owner: Pubkey,
    to_vault: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    from_vault_authority_seed: u8,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> ProgramResult {
    let mode = pda::PDA::StabilityPool { owner };
    spltoken::transfer_from_vault(
        amount,
        mode,
        to_vault,
        from_vault,
        from_vault_authority,
        from_vault_authority_seed,
        token_program,
        program_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_borrowing_fees_vault<'info>(
    amount: u64,
    owner: Pubkey,
    to_vault: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    from_vault_authority_seed: u8,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> ProgramResult {
    msg!(
        "Transferring {} stablecoin from {} to {}",
        amount.to_string(),
        from_vault.clone().key.to_string(),
        to_vault.clone().key.to_string(),
    );

    let mode = pda::PDA::BorrowingFeesAccount { owner };
    spltoken::transfer_from_vault(
        amount,
        mode,
        to_vault,
        from_vault,
        from_vault_authority,
        from_vault_authority_seed,
        token_program,
        program_id,
    )
}
