use crate::BorrowError;
use crate::{Price, TokenPrices};
use anchor_lang::prelude::{AccountInfo, ProgramError};
use pyth_client::PriceStatus;

pub fn get_prices(
    pyth_sol_price_info: &AccountInfo,
    pyth_eth_price_info: &AccountInfo,
    pyth_btc_price_info: &AccountInfo,
    pyth_srm_price_info: &AccountInfo,
    pyth_ray_price_info: &AccountInfo,
    pyth_ftt_price_info: &AccountInfo,
) -> Result<TokenPrices, ProgramError> {
    // sol: Price::from(22841550900, 8),
    // eth: Price::from(472659830000, 8),
    // btc: Price::from(6462236900000, 8),
    // srm: Price::from(706975570, 8),
    // ftt: Price::from(5917104600, 8),
    // ray: Price::from(1110038050, 8),
    Ok(TokenPrices {
        sol: get_price(pyth_sol_price_info)?,
        eth: get_price(pyth_eth_price_info)?,
        btc: get_price(pyth_btc_price_info)?,
        srm: get_price(pyth_srm_price_info)?,
        ray: get_price(pyth_ray_price_info)?,
        ftt: get_price(pyth_ftt_price_info)?,
    })
}

pub fn get_price(pyth_price_info: &AccountInfo) -> Result<Price, ProgramError> {
    // if pyth_product.magic != pyth_client::MAGIC {
    //     msg!("Pyth product account provided is not a valid Pyth account");
    //     return Err(ProgramError::InvalidArgument);
    // }
    // if pyth_product.atype != pyth_client::AccountType::Product as u32 {
    //     msg!("Pyth product account provided is not a valid Pyth product account");
    //     return Err(ProgramError::InvalidArgument);
    // }
    // if pyth_product.ver != pyth_client::VERSION_2 {
    //     msg!("Pyth product account provided has a different version than the Pyth client");
    //     return Err(ProgramError::InvalidArgument);
    // }
    // if !pyth_product.px_acc.is_valid() {
    //     msg!("Pyth product price account is invalid");
    //     return Err(ProgramError::InvalidArgument);
    // }

    // let pyth_price_pubkey = Pubkey::new(&pyth_product.px_acc.val);
    // if &pyth_price_pubkey != pyth_price_info.key {
    //     msg!("Pyth product price account does not match the Pyth price provided");
    //     return Err(ProgramError::InvalidArgument);
    // }

    // eth -8 472659830000
    // ray -8 1110038050
    // srm -8 706975570
    // btc -8 6462236900000
    // sol -8 22841550900
    // ftt -8 5917104600

    let pyth_price_data = &pyth_price_info.try_borrow_data()?;
    let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);
    let is_trading = get_status(&pyth_price.agg.status);
    if !is_trading {
        return Err(BorrowError::PriceNotValid.into());
    }

    Ok(Price::from(
        pyth_price.agg.price as u64,
        pyth_price.expo.abs() as u8,
    ))
}

fn get_status(st: &PriceStatus) -> bool {
    matches!(st, PriceStatus::Trading)
}
