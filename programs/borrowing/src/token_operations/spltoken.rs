use anchor_lang::{
    prelude::{msg, AccountInfo, ProgramResult, Pubkey},
    CpiContext,
};
use anchor_spl::token::{self, Burn, MintTo, Transfer};

use crate::utils::pda::{self};

#[allow(clippy::too_many_arguments)]
pub fn mint<'info>(
    mint_coin: &AccountInfo<'info>,
    mint_to: &AccountInfo<'info>,
    mint_coin_authority: &AccountInfo<'info>,
    mint_coin_authority_seed: u8,
    pda_mode: crate::pda::PDA,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
    amount: u64,
) -> ProgramResult {
    let mint_seed: u8 = mint_coin_authority_seed;
    let mint_seeds = vec![mint_seed];

    let reward_mint_pda_seeds = crate::pda::make_pda_seeds(&pda_mode, program_id);
    let seeds = [
        reward_mint_pda_seeds[0].as_ref(),
        reward_mint_pda_seeds[1].as_ref(),
        mint_seeds.as_ref(),
    ];
    let signer = &[&seeds[..]];
    let cpi_mint_accounts = MintTo {
        mint: mint_coin.clone(),
        to: mint_to.clone(),
        authority: mint_coin_authority.clone(),
    };
    let cpi_mint_program = token_program.clone();
    let cpi_ctx = CpiContext::new(cpi_mint_program, cpi_mint_accounts).with_signer(signer);

    let result = token::mint_to(cpi_ctx, amount);
    msg!(
        "Minted {} to {} with result {:?}",
        amount,
        mint_to.key,
        result
    );

    result
}

#[allow(clippy::too_many_arguments)]
pub fn burn<'info>(
    mint: &AccountInfo<'info>,
    burn_from: &AccountInfo<'info>,
    burn_authority: &AccountInfo<'info>,
    burn_authority_seed: u8,
    pda_mode: crate::pda::PDA,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
    amount: u64,
) -> ProgramResult {
    let seed = vec![burn_authority_seed];
    let burn_authority_pda_seeds = crate::pda::make_pda_seeds(&pda_mode, program_id);
    let seeds = [
        burn_authority_pda_seeds[0].as_ref(),
        burn_authority_pda_seeds[1].as_ref(),
        seed.as_ref(),
    ];

    let signer = &[&seeds[..]];

    let cpi_mint_accounts = Burn {
        mint: mint.clone(),
        to: burn_from.clone(),
        authority: burn_authority.clone(),
    };
    let cpi_program = token_program.clone();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_mint_accounts).with_signer(signer);

    let result = token::burn(cpi_ctx, amount);
    msg!("Burned {:?}", result);
    result
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_vault<'info>(
    amount: u64,
    mode: pda::PDA,
    to_vault: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    from_vault_authority_seed: u8,
    token_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> ProgramResult {
    let from_vault_seed: u8 = from_vault_authority_seed;
    let from_vault_authority_bump = vec![from_vault_seed];
    let from_vault_authority_pda_seeds = pda::make_pda_seeds(&mode, program_id);
    let seeds = [
        from_vault_authority_pda_seeds[0].as_ref(),
        from_vault_authority_pda_seeds[1].as_ref(),
        from_vault_authority_bump.as_ref(),
    ];
    let signer = &[&seeds[..]];

    let cpi_transfer_accounts = Transfer {
        from: from_vault.clone(),
        to: to_vault.clone(),
        authority: from_vault_authority.clone(),
    };

    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_transfer_accounts).with_signer(signer);
    token::transfer(cpi_ctx, amount)
}

pub fn transfer_from_user<'info>(
    amount: u64,
    from_ata: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> ProgramResult {
    let cpi_transfer_accounts = Transfer {
        from: from_ata.clone(),
        to: to.clone(),
        authority: authority.clone(),
    };
    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_transfer_accounts);

    let result = token::transfer(cpi_ctx, amount);
    msg!("Transfered {:?}", result);
    result
}
