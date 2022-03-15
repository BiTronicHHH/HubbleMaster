import { BN } from "@project-serum/anchor";
import {
    Market,
    DexInstructions,
} from "@project-serum/serum";
import {
    Account,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from "@solana/web3.js";
import {
    Token,
    TOKEN_PROGRAM_ID,
    AccountLayout as TokenAccountLayout,
    NATIVE_MINT,
} from "@solana/spl-token";
import * as anchor from "@project-serum/anchor";
import * as assert from "assert";
import { DEX_PROGRAM_ID } from "./global";
import { createAccountIx } from "./utils";

export interface MarketInfo {
    baseToken: Token;
    quoteToken: Token;
    baseLotSize: number;
    quoteLotSize: number;
    feeRateBps: number;
}

export interface MarketMaker {
    account: Account;
    tokenAccounts: { [mint: string]: PublicKey };
}

export interface PubkeyDictionary {
    [Key: string]: PublicKey;
}


export async function createMarket(
    provider: anchor.Provider,
    signer: Signer,
    info: MarketInfo
): Promise<{
    market: Market;
    requestQueue: PublicKey;
    eventQueue: PublicKey;
    baseVault: PublicKey;
    quoteVault: PublicKey;
    vaultOwner: PublicKey;
}> {
    const market = Keypair.generate();
    const requestQueue = Keypair.generate();
    const eventQueue = Keypair.generate();
    const bids = Keypair.generate();
    const asks = Keypair.generate();
    const quoteDustThreshold = new BN(100);

    const [vaultOwner, vaultOwnerBump] = await findVaultOwner(market.publicKey);

    const [baseVault, quoteVault] = await Promise.all([
        createTokenAccount(provider, info.baseToken, vaultOwner, signer, new BN(0)),
        createTokenAccount(
            provider,
            info.quoteToken,
            vaultOwner,
            signer,
            new BN(0)
        ),
    ]);

    const initMarketTx = new Transaction();
    initMarketTx.add(
        await createAccountIx(
            provider,
            market.publicKey,
            Market.getLayout(DEX_PROGRAM_ID).span,
            signer,
            DEX_PROGRAM_ID
        ),
        await createAccountIx(
            provider,
            requestQueue.publicKey,
            5132,
            signer,
            DEX_PROGRAM_ID
        ),
        await createAccountIx(
            provider,
            eventQueue.publicKey,
            262156,
            signer,
            DEX_PROGRAM_ID
        ),
        await createAccountIx(
            provider,
            bids.publicKey,
            65548,
            signer,
            DEX_PROGRAM_ID
        ),
        await createAccountIx(
            provider,
            asks.publicKey,
            65548,
            signer,
            DEX_PROGRAM_ID
        ),
        DexInstructions.initializeMarket(
            {
                market: market.publicKey,
                requestQueue: requestQueue.publicKey,
                eventQueue: eventQueue.publicKey,
                bids: bids.publicKey,
                asks: asks.publicKey,
                baseVault,
                quoteVault,
                baseMint: info.baseToken.publicKey,
                quoteMint: info.quoteToken.publicKey,
                baseLotSize: new BN(info.baseLotSize),
                quoteLotSize: new BN(info.quoteLotSize),
                feeRateBps: info.feeRateBps,
                vaultSignerNonce: vaultOwnerBump,
                quoteDustThreshold,
                programId: DEX_PROGRAM_ID,
            })
    );

    initMarketTx.feePayer = provider.wallet.publicKey;

    await sendAndConfirmTransaction(provider.connection, initMarketTx, [
        market,
        requestQueue,
        eventQueue,
        bids,
        asks,
        signer,
    ]);

    return {
        market: await Market.load(
            provider.connection,
            market.publicKey,
            undefined, // commitment - recent
            DEX_PROGRAM_ID
        ),
        requestQueue: requestQueue.publicKey,
        eventQueue: eventQueue.publicKey,
        baseVault,
        quoteVault,
        vaultOwner,
    };
}

export async function createMarketMaker(
    provider: anchor.Provider,
    signer: Account,
    lamports: number,
    tokens: [Token, BN][]
): Promise<MarketMaker> {
    const wallet = signer;

    const fundTx = new Transaction().add(
        SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: wallet.publicKey,
            lamports,
        })
    );
    await sendAndConfirmTransaction(provider.connection, fundTx, [signer]);
    const tokenAccounts: PubkeyDictionary = {};

    for (const [token, amount] of tokens) {
        const publicKey = await createTokenAccount(
            provider,
            token,
            wallet.publicKey,
            signer,
            amount
        );

        tokenAccounts[token.publicKey.toBase58()] = publicKey;
    }

    return {
        account: wallet,
        tokenAccounts,
    };
}

export interface Order {
    price: number;
    size: number;
}

export function makeOrders(orders: [number, number][]): Order[] {
    return orders.map(([price, size]) => ({ price, size }));
}

export async function placeOrders(
    provider: anchor.Provider,
    marketMaker: MarketMaker,
    market: Market,
    bids: Order[],
    asks: Order[],
) {
    const baseTokenAccount =
        marketMaker.tokenAccounts[market.baseMintAddress.toBase58()];
    const quoteTokenAccount =
        marketMaker.tokenAccounts[market.quoteMintAddress.toBase58()];

    let marketMakerUsdcBalanceBefore = await (
        await provider.connection.getTokenAccountBalance(quoteTokenAccount)
    ).value.uiAmount;
    let marketMakerBtcBalanceBefore = await (
        await provider.connection.getTokenAccountBalance(baseTokenAccount)
    ).value.uiAmount;

    const placeOrderDefaultParams = {
        owner: marketMaker.account,
        clientId: undefined,
        openOrdersAddressKey: undefined,
        openOrdersAccount: undefined,
        feeDiscountPubkey: null,
    };

    let btcSum = 0;
    let usdcSum = 0;

    for (const entry of asks) {
        let price = parseFloat(entry.price.toFixed(3));
        console.log("In asks at ", price, entry.size);
        const { transaction, signers } = await market.makePlaceOrderTransaction(
            provider.connection,
            {
                payer: baseTokenAccount,
                side: "sell",
                price: entry.price,
                size: entry.size,
                orderType: "postOnly",
                selfTradeBehavior: "abortTransaction",
                ...placeOrderDefaultParams,
            }
        );
        btcSum += entry.size;
        await provider.send(transaction, signers.concat(marketMaker.account));
    }

    for (const entry of bids) {
        let price = parseFloat(entry.price.toFixed(3));
        console.log("In bids at ", price, entry.size);
        const { transaction, signers } = await market.makePlaceOrderTransaction(
            provider.connection,
            {
                payer: quoteTokenAccount,
                side: "buy",
                price: entry.price,
                size: entry.size,
                orderType: "postOnly",
                selfTradeBehavior: "abortTransaction",
                ...placeOrderDefaultParams,
            }
        );
        usdcSum += entry.size * entry.price;
        await provider.send(transaction, signers.concat(marketMaker.account));
    }

    let marketMakerUsdcBalanceAfter = await (
        await provider.connection.getTokenAccountBalance(quoteTokenAccount)
    ).value.uiAmount;
    let marketMakerBtcBalanceAfter = await (
        await provider.connection.getTokenAccountBalance(baseTokenAccount)
    ).value.uiAmount;

    //@ts-ignore
    assert.strictEqual(marketMakerBtcBalanceAfter, marketMakerBtcBalanceBefore - btcSum);

    // Small decimal loss imprecision at the order placement for the market maker, taking into account the makers fee (3%)
    //@ts-ignore
    console.log("Makers fee and precision loss", marketMakerUsdcBalanceBefore - marketMakerUsdcBalanceAfter - usdcSum);
    console.log("USDC SUM", usdcSum);
    console.log("marketMakeUsdcBalanceBefore", marketMakerUsdcBalanceBefore);
    console.log("marketMakeUsdcBalanceAfter", marketMakerUsdcBalanceAfter);
    //@ts-ignore
    assert.ok((marketMakerUsdcBalanceBefore - marketMakerUsdcBalanceAfter - usdcSum) < 0.0051);
}

export async function createTokenAccount(
    provider: anchor.Provider,
    token: Token,
    owner: PublicKey,
    signer: Signer,
    amount: BN
): Promise<PublicKey> {
    if (token.publicKey == NATIVE_MINT) {
        const account = await Token.createWrappedNativeAccount(
            provider.connection,
            TOKEN_PROGRAM_ID,
            owner,
            signer,
            amount.toNumber()
        );
        return account;
    } else {
        const account = await token.createAccount(owner);
        await token.mintTo(account, signer, [], amount.toNumber());
        return account;
    }
}

async function findVaultOwner(market: PublicKey): Promise<[PublicKey, BN]> {
    const bump = new BN(0);

    while (bump.toNumber() < 255) {
        try {
            const vaultOwner = await PublicKey.createProgramAddress(
                [market.toBuffer(), bump.toArrayLike(Buffer, "le", 8)],
                DEX_PROGRAM_ID
            );

            return [vaultOwner, bump];
        } catch (_e) {
            bump.iaddn(1);
        }
    }

    throw new Error("no seed found for vault owner");
}

export async function createToken(
    provider: anchor.Provider,
    decimals: number,
    authority: PublicKey,
    signer: Signer
): Promise<Token> {
    const token = await Token.createMint(
        provider.connection,
        signer,
        authority,
        authority,
        decimals,
        TOKEN_PROGRAM_ID
    );

    return token;
}

export async function createNativeToken(
    provider: anchor.Provider,
    signer: Signer
) {
    const token = new Token(
        provider.connection,
        NATIVE_MINT,
        TOKEN_PROGRAM_ID,
        signer
    );
    return token;
}

