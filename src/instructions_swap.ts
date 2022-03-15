import * as anchor from "@project-serum/anchor";
import {
    PublicKey,
    Signer,
} from "@solana/web3.js";
import * as global from "../src/global";
import { Token as SplToken, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BorrowingGlobalAccounts, PythPrices, setUpAta } from "./set_up";


import {
    OpenOrders,
} from "@project-serum/serum";
import { CollateralToken, collateralTokenToNumber, UserMetadata } from "../tests/types";

export async function serumInitAccount(
    program: anchor.Program,
    openOrders: Signer,
    market: PublicKey,
    dexProgram: PublicKey,
    authority: PublicKey,
    signer: Signer,
) {
    const tx = await program.rpc.serumInitAccount({
        accounts: {
            openOrders: openOrders.publicKey,
            market,
            orderPayerAuthority: authority,
            dexProgram,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        instructions: [
            await OpenOrders.makeCreateAccountTransaction(
                program.provider.connection,
                market,
                signer.publicKey,
                openOrders.publicKey,
                dexProgram
            ),
        ],
        signers: [openOrders, signer],
    });
    console.log("Initialized open orders account for trading", tx);
}

export async function swapToUsdc(
    provider: anchor.Provider,
    program: anchor.Program,
    marketAddress: PublicKey,
    openOrders: PublicKey,
    requestQueue: PublicKey,
    eventQueue: PublicKey,
    baseVault: PublicKey,
    quoteVault: PublicKey,
    vaultOwner: PublicKey,
    quoteTokenAccount: PublicKey,
    bidsAddress: PublicKey,
    asksAddress: PublicKey,
    token: CollateralToken = "SOL",
    borrowingAccounts: BorrowingGlobalAccounts,
    baseAmount: number,
    user: Signer,
    userMetadata: PublicKey,
    pythPrices: PythPrices,
    usdcMint: PublicKey
) {
    let borrowingVaultsAccount = await program.account.borrowingVaults.fetch(
        borrowingAccounts.borrowingVaults.publicKey
    );
    let collateralVaultAuthority = new PublicKey(
        // @ts-ignore
        borrowingVaultsAccount.collateralVaultsAuthority
    );
    let collateralVault, dex_swap_account, pythTokenPriceInfo;


    switch (token) {
        case "SOL":
            {
                collateralVault = borrowingAccounts.collateralVaultSol;
                //TODO: add Wsol
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.btcMint
                );
                pythTokenPriceInfo = pythPrices.solPythPrice.publicKey;
            }
            break;
        case "BTC":
            {
                collateralVault = borrowingAccounts.collateralVaultBtc;
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.btcMint
                );

                pythTokenPriceInfo = pythPrices.btcPythPrice.publicKey;
            }
            break;
        case "ETH":
            {
                collateralVault = borrowingAccounts.collateralVaultEth;
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.ethMint
                );
                pythTokenPriceInfo = pythPrices.ethPythPrice.publicKey;
            }
            break;
        case "SRM":
            {
                collateralVault = borrowingAccounts.collateralVaultSrm;
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.srmMint
                );
                pythTokenPriceInfo = pythPrices.srmPythPrice.publicKey;
            }
            break;
        case "RAY":
            {
                collateralVault = borrowingAccounts.collateralVaultRay;
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.rayMint
                );
                pythTokenPriceInfo = pythPrices.rayPythPrice.publicKey;
            }
            break;
        case "FTT":
            {
                collateralVault = borrowingAccounts.collateralVaultFtt;
                dex_swap_account = await setUpAta(
                    provider,
                    user,
                    borrowingAccounts.fttMint
                );
                pythTokenPriceInfo = pythPrices.fttPythPrice.publicKey;
            }
            break;
    }
    let borrowingMarketState = borrowingAccounts.borrowingMarketState;
    let borrowingVaults = borrowingAccounts.borrowingVaults;

    const tx = await program.rpc.serumSwapUsdc(0, new anchor.BN(baseAmount), new anchor.BN(collateralTokenToNumber(token)), {
        accounts: {
            dexProgram: global.DEX_PROGRAM_ID,
            market: marketAddress,
            openOrders: openOrders,
            owner: user.publicKey,
            requestQueue,
            eventQueue,
            bids: bidsAddress,
            asks: asksAddress,
            coinVault: baseVault,
            pcVault: quoteVault,
            dexSwapAccount: dex_swap_account,
            pcWallet: quoteTokenAccount,
            vaultSigner: vaultOwner,
            collateralVault, // the account that receives the base tokens back, if the orders aren't matched
            borrowingMarketState: borrowingMarketState.publicKey,
            borrowingVaults: borrowingVaults.publicKey,
            userMetadata: userMetadata,
            collateralFromAuthority: collateralVaultAuthority,
            pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
            pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
            pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
            pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
            pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
            pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
            usdcMint,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [user]
    });
    console.log(
        `Serum swap to usdc gains tx signature ${tx}`
    );
}

export async function serumCloseAccount(
    program: anchor.Program,
    openOrders: Signer,
    market: PublicKey,
    dexProgram: PublicKey,
    authority: PublicKey,
    signer: Signer,
) {
    const tx = await program.rpc.serumCloseAccount({
        accounts: {
            openOrders: openOrders.publicKey,
            market,
            authority,
            destination: authority,
            dexProgram,
        },
        signers: [signer],
    });
    console.log("Closed open orders account for trading", tx);
}