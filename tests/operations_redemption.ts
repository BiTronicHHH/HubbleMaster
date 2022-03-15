import * as anchor from "@project-serum/anchor";
import { PublicKey, TransactionSignature } from "@solana/web3.js";
import { BorrowingGlobalAccounts, BorrowingUserState, Env, PythPrices } from "../src/set_up";
import * as instructions_redeem from '../src/instructions_redeem';
import { newBorrowingUser } from "./operations_borrowing";
import { CollateralToken } from "./types";
import { displayUserBalances } from "../src/utils_display";
import { airdropStablecoin } from "../src/instructions_borrow";
import * as utils from "../src/utils";

export async function newRedemptionUser(
    env: Env,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    stablecoinBalance: number,
    minSolBalance: number,
): Promise<BorrowingUserState> {
    const user = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
        ["SOL", minSolBalance]
    ]));
    await airdropStablecoin(env.program, env.provider.wallet.publicKey,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        user.borrowerAccounts.stablecoinAta, borrowingGlobalAccounts.stablecoinMint,
        utils.decimalToU64(stablecoinBalance),
    )
    console.log(`Created redemption user -> ${user.borrower.publicKey.toBase58()}`);
    console.log(`   UserMetadata -> ${user.borrowerAccounts.userMetadata.publicKey.toBase58()}`);
    await displayUserBalances(env.provider, user.borrower.publicKey, borrowingGlobalAccounts, user.borrowerAccounts);
    return user;
}

export async function newFillUser(
    env: Env,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
): Promise<BorrowingUserState> {
    const user = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
        ["SOL", 1]
    ]));
    console.log(`Created redemption fill user -> ${user.borrower.publicKey.toBase58()}`);
    console.log(`   UserMetadata -> ${user.borrowerAccounts.userMetadata.publicKey.toBase58()}`);
    return user;
}

export async function newFillUsers(
    env: Env,
    globalAccounts: BorrowingGlobalAccounts,
    numberOfUsers: number
): Promise<Array<BorrowingUserState>> {
    console.log(`Creating ${numberOfUsers} fill users...`)
    const promises = new Array<Promise<BorrowingUserState>>();
    for (let i = 0; i < numberOfUsers; i++) {
        promises.push(newFillUser(env, globalAccounts));
    }
    return Promise.all(promises);
}

export async function newClearUser(
    env: Env,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
): Promise<BorrowingUserState> {
    const user = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
        ["SOL", 1]
    ]));
    console.log(`Created redemption clear user -> ${user.borrower.publicKey.toBase58()}`);
    console.log(`   UserMetadata -> ${user.borrowerAccounts.userMetadata.publicKey.toBase58()}`);
    return user;
}

export async function add_redemption_order(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    redemptionUser: BorrowingUserState,
    pythPrices: PythPrices,
    redeemStablecoin: number,
): Promise<TransactionSignature> {
    return instructions_redeem.addRedemptionOrder(
        program,
        redemptionUser.borrower,
        redemptionUser.borrowerAccounts.userMetadata.publicKey,
        redemptionUser.borrowerAccounts.stablecoinAta,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.borrowingVaults.publicKey,
        borrowingGlobalAccounts.redemptionsQueue,
        borrowingGlobalAccounts.burningVault,
        pythPrices,
        utils.decimalToU64(redeemStablecoin),
    );
}

export async function fill_redemption_order(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    filler: BorrowingUserState,
    orderId: number,
    candidateMetadata: PublicKey[],
): Promise<TransactionSignature> {
    return instructions_redeem.fillRedemptionOrder(
        program,
        filler.borrower,
        filler.borrowerAccounts.userMetadata.publicKey,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.redemptionsQueue,
        orderId,
        candidateMetadata,
    );
}

export async function clear_redemption_order(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    clearer: BorrowingUserState,
    redeemer: BorrowingUserState,
    orderId: number,
    borrowerAndFillerMetadatas: PublicKey[],
): Promise<TransactionSignature> {
    return instructions_redeem.clearRedemptionOrder(
        program,
        clearer.borrower,
        clearer.borrowerAccounts.userMetadata.publicKey,
        redeemer.borrowerAccounts.userMetadata.publicKey,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.borrowingVaults.publicKey,
        borrowingGlobalAccounts.redemptionsQueue,
        borrowingGlobalAccounts.burningVault,
        borrowingGlobalAccounts.stablecoinMint,
        orderId,
        borrowerAndFillerMetadatas,
    );
}
