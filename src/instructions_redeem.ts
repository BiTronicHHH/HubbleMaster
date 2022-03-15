import * as anchor from "@project-serum/anchor";
import { Keypair, PublicKey, Transaction, TransactionInstruction, TransactionSignature } from "@solana/web3.js";
import * as utils from "./utils";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PythPrices } from "./set_up";
import { getBorrowingVaults } from "../tests/data_provider";
import { CLEAR_INST_METADATA_ACCS_SIZE, FILL_INST_METADATA_ACCS_SIZE } from "../tests/tests_redemption";
import { mapAnchorError, publicKeyReplacer } from "./utils";

export async function addRedemptionOrder(
    program: anchor.Program,
    redeemer: Keypair,
    redeemerMetadata: PublicKey,
    redeemerStablecoinAssociatedAccount: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    redemptionsQueue: PublicKey,
    burningVault: PublicKey,
    pythPrices: PythPrices,
    redeemStablecoin: number,
): Promise<TransactionSignature> {

    const accounts = getAddRedemptionOrderAccounts(
        redeemer,
        redeemerMetadata,
        redeemerStablecoinAssociatedAccount,
        borrowingMarketState,
        borrowingVaults,
        redemptionsQueue,
        burningVault,
        pythPrices
    );

    console.log(`Adding redemption order for ${utils.u64ToDecimal(redeemStablecoin)} stablecoin...\n${JSON.stringify(accounts, publicKeyReplacer, 2)}`);

    const txid = await mapAnchorError(program.rpc.addRedemptionOrder(
        new anchor.BN(redeemStablecoin), {
        accounts: accounts,
        signers: [redeemer]
    }));

    console.log(`Add redemption order transaction signature: ${txid}`);

    return txid;
}

export function getAddRedemptionOrderAccounts(redeemer: Keypair, redeemerMetadata: PublicKey, redeemerStablecoinAssociatedAccount: PublicKey, borrowingMarketState: PublicKey, borrowingVaults: PublicKey, redemptionsQueue: PublicKey, burningVault: PublicKey, pythPrices: PythPrices) {
    return {
        redeemer: redeemer.publicKey,
        redeemerMetadata,
        redeemerStablecoinAssociatedAccount,
        borrowingMarketState,
        borrowingVaults,
        redemptionsQueue,
        burningVault,
        pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
        pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
        pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
        pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
        pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
        pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    };
}

export async function fillRedemptionOrder(
    program: anchor.Program,
    filler: Keypair,
    fillerMetadata: PublicKey,
    borrowingMarketState: PublicKey,
    redemptionsQueue: PublicKey,
    orderId: number,
    candidateMetadatas: PublicKey[],
): Promise<TransactionSignature> {

    console.log('Filling redemption order...');
    let metadataAccounts: any = getMetadataAccounts(candidateMetadatas, FILL_INST_METADATA_ACCS_SIZE);

    console.log()

    const txid: TransactionSignature = await mapAnchorError(program.rpc.fillRedemptionOrder(
        new anchor.BN(orderId), {
        accounts: getFillRedemptionOrderAccounts(
            filler,
            fillerMetadata,
            borrowingMarketState,
            redemptionsQueue
        ),
        remainingAccounts: metadataAccounts,
        signers: [filler]
    }));

    console.log(`Fill redemption order transaction signature: ${txid}`);

    return txid;
}

export function getFillRedemptionOrderAccounts(
    filler: Keypair,
    fillerMetadata: PublicKey,
    borrowingMarketState: PublicKey,
    redemptionsQueue: PublicKey
) {
    return {
        filler: filler.publicKey,
        fillerMetadata,
        borrowingMarketState,
        redemptionsQueue,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    };
}

export async function clearRedemptionOrder(
    program: anchor.Program,
    clearer: Keypair,
    clearerMetadata: PublicKey,
    redeemerMetadata: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    redemptionsQueue: PublicKey,
    burningVault: PublicKey,
    stablecoinMint: PublicKey,
    orderId: number,
    borrowerAndFillerMetadatas: PublicKey[],
): Promise<TransactionSignature> {

    console.log('Clearing redemption order...');

    const { burningVaultAuthority } = await getBorrowingVaults(program, borrowingVaults);

    let metadataAccounts: any = getMetadataAccounts(borrowerAndFillerMetadatas, CLEAR_INST_METADATA_ACCS_SIZE);

    const txid: TransactionSignature = await mapAnchorError(program.rpc.clearRedemptionOrder(
        new anchor.BN(orderId), {
        accounts: getClearRedemptionOrderAccounts(
            clearer,
            clearerMetadata,
            redeemerMetadata,
            borrowingMarketState,
            borrowingVaults,
            redemptionsQueue,
            burningVault,
            burningVaultAuthority,
            stablecoinMint
        ),
        remainingAccounts: metadataAccounts,
        signers: [clearer]
    }));

    console.log(`Clear redemption order transaction signature: ${txid}`);

    return txid;
}

export function getClearRedemptionOrderAccounts(
    clearer: Keypair,
    clearerMetadata: PublicKey,
    redeemerMetadata: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    redemptionsQueue: PublicKey,
    burningVault: PublicKey,
    burningVaultAuthority: PublicKey,
    stablecoinMint: PublicKey,
): any {
    return {
        clearer: clearer.publicKey,
        clearerMetadata,
        redeemerMetadata,
        borrowingMarketState,
        borrowingVaults,
        redemptionsQueue,
        burningVault,
        burningVaultAuthority,
        stablecoinMint,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
    }
}


export function getMetadataAccounts(metadatas: PublicKey[], maximumMetadataAccounts: number): any[] {
    let accounts = [];
    for (let i = 0; i < Math.min(maximumMetadataAccounts, metadatas.length); i++) {
        accounts.push({
            pubkey: metadatas[i],
            isWritable: true,
            isSigner: false
        });
    }
    return accounts;
}
