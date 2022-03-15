import * as anchor from "@project-serum/anchor";
import {
    Account,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
} from "@solana/web3.js";

import * as global from "../src/global";
import * as set_up from "../src/set_up";
import * as utils from "../src/utils";
import * as operations_borrowing from "./operations_borrowing";
import * as instructions_swap from "../src/instructions_swap";
import * as instructions_borrow from "../src/instructions_borrow";
import { setUpProgram } from "../src/set_up";

import * as web3 from "@solana/web3.js";
import * as splToken from "@solana/spl-token";
import * as serumUtils from "../src/utils_serum";
import { CollateralToken } from "./types";

import * as assert from "assert";

import { sleep } from "@project-serum/common";
import { PythUtils } from "../src/pyth";
import {
    Token as SplToken,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { getBorrowingMarketState, getUserMetadata } from "./data_provider";

import * as chai from 'chai'
import { expect } from "chai";
import chaiAsPromised from 'chai-as-promised'
import { collToLamports } from "../src/utils";

chai.use(chaiAsPromised)

describe("serum_swap", () => {

    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();

    // Use a keypair to create USDC Token and Serum Market (will be already deployed in prod)
    const keypair_acc = Uint8Array.from(Buffer.from(JSON.parse(require("fs").readFileSync("./keypair.json"))));
    // using account for legacy createToken functions
    const owner = new anchor.web3.Account(keypair_acc);

    // console.log("owner", owner.publicKey.toBase58());


    const solPrice = 228.49
    const ethPrice = 4726.59;
    const btcPrice = 64622.369;
    const srmPrice = 7.06;
    const fttPrice = 59.17;
    const rayPrice = 11.10;


    it("tests_swap_btc", async () => {
        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        const btcSwap = 20;

        let { borrowingGlobalAccounts, pythPrices, assetMint: btcMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, initialMarketOwner, pyth, depositBtc, "BTC");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, btcMint, btcPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        // Assert balances before the order placement
        let { userUsdcBalance,
            baseVaultBalance: baseVaultBalanceBefore,
            quoteVaultBalance: quoteVaultBalanceBefore,
            // @ts-ignore
            collateralVaultBalance: collateralVaultBalanceBefore } = await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, borrowingGlobalAccounts.collateralVaultBtc, 0, marketMakerBtcInitialBalance - marketMakerBaseTokenBalance, marketMakerUsdcInitialBalance - marketMakerQuoteTokenBalance, depositBtc);

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            user.publicKey,
            user
        );


        await instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            collToLamports(btcSwap, "BTC"),
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        );

        await sleep(3000);

        let userMetadataAfter = await getUserMetadata(program, userAccounts.userMetadata.publicKey);
        assert.strictEqual(userMetadataAfter.inactiveCollateral.btc, collToLamports(depositBtc - btcSwap, "BTC"));
        assert.strictEqual(userMetadataAfter.borrowedStablecoin, 0);

        const { inactiveCollateral } = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        assert.strictEqual(inactiveCollateral.btc, collToLamports(depositBtc - btcSwap, "BTC"));

        // The USDC received, considering getting the best orders on the serum market
        let expectedOrderSize = orderSize * btcPrice * (1 - orderIncrements) + orderSize * btcPrice * (1 - 2 * orderIncrements);

        // @ts-ignore
        await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, borrowingGlobalAccounts.collateralVaultBtc, expectedOrderSize, baseVaultBalanceBefore + btcSwap, quoteVaultBalanceBefore, collateralVaultBalanceBefore - btcSwap);

    });

    it("tests_swap_0_tokens_error", async () => {
        const btcPrice = 64622.369;

        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        let { borrowingGlobalAccounts, pythPrices, assetMint: btcMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, provider.wallet.publicKey, pyth, depositBtc, "BTC");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, btcMint, btcPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            user.publicKey,
            user
        );

        await expect(instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            0,
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        ), `Attempting to swap 0 was not rejected`).to.be.rejected;

    });

    it("tests_swap_no_deposit", async () => {
        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        const btcSwap = 20;

        const pythPrices = await set_up.setUpPythPrices(
            provider,
            pyth,
        );

        let { user: alice, userAccounts: aliceAccounts, borrowingGlobalAccounts } =
            await operations_borrowing.setUpMarketWithEmptyUser(
                provider,
                program,
                initialMarketOwner
            );

        let btcMint = borrowingGlobalAccounts.btcMint;

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );
        const bob = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(bob.publicKey, collToLamports(10, "SOL"));

        await sleep(2000);

        const bobAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            bob.publicKey,
            [bob],
            bob.publicKey,
            borrowingGlobalAccounts);

        console.log(`Created User Accounts for BOB`);

        await instructions_borrow
            .initializeTrove(
                program,
                bob.publicKey,
                bobAccounts.userMetadata,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                bobAccounts.stablecoinAta,
                [bob]);
        await operations_borrowing.mintToAta(
            provider,
            borrowingGlobalAccounts,
            bobAccounts,
            "BTC",
            collToLamports(depositBtc, "BTC")
        );


        // deposit 500 BTC
        await instructions_borrow.depositCollateral(
            program,
            bob.publicKey,
            bobAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts.collateralVaultBtc,
            bobAccounts.btcAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            collToLamports(depositBtc, "BTC"),
            [bob],
            "BTC"
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, btcMint, btcPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            alice.publicKey,
            [alice],
            alice.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = aliceAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            alice.publicKey,
            alice
        );

        await expect(instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            collToLamports(btcSwap, "BTC"),
            alice,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        ), `User swaps with no deposit was no rejected`).to.be.rejected;


    });

    it("tests_swap_eth", async () => {
        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        const btcSwap = 20;


        let { borrowingGlobalAccounts, pythPrices, assetMint: ethMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, provider.wallet.publicKey, pyth, depositBtc, "ETH");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, ethMint, ethPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        // Assert balances before the order placement
        let { userUsdcBalance: userUsdcBalanceBefore,
            baseVaultBalance: baseVaultBalanceBefore,
            quoteVaultBalance: quoteVaultBalanceBefore,
            // @ts-ignore
            collateralVaultBalance: collateralVaultBalanceBefore } = await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, borrowingGlobalAccounts.collateralVaultEth, 0, marketMakerBtcInitialBalance - marketMakerBaseTokenBalance, marketMakerUsdcInitialBalance - marketMakerQuoteTokenBalance, depositBtc);

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            user.publicKey,
            user
        );


        await instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "ETH",
            borrowingGlobalAccounts,
            collToLamports(btcSwap, "BTC"),
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        );

        await sleep(3000);

        let userMetadataAfter = await getUserMetadata(program, userAccounts.userMetadata.publicKey);
        assert.strictEqual(userMetadataAfter.inactiveCollateral.eth, collToLamports(depositBtc - btcSwap, "ETH"));
        assert.strictEqual(userMetadataAfter.borrowedStablecoin, 0);

        const { inactiveCollateral } = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        assert.strictEqual(inactiveCollateral.eth, collToLamports(depositBtc - btcSwap, "ETH"));

        // The USDC received, considering getting the best orders on the serum market
        let expectedOrderSize = orderSize * ethPrice * (1 - orderIncrements) + orderSize * ethPrice * (1 - 2 * orderIncrements);

        // @ts-ignore
        await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, borrowingGlobalAccounts.collateralVaultEth, expectedOrderSize, baseVaultBalanceBefore + btcSwap, quoteVaultBalanceBefore, collateralVaultBalanceBefore - btcSwap);

    });

    it("tests_swap_multiple_markets", async () => {
        const prices = [solPrice, ethPrice, btcPrice, srmPrice, rayPrice, fttPrice];

        const depositAsset = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        const assetSwap = 20;

        const collateral: CollateralToken[] = ["SOL", "ETH", "BTC", "SRM", "RAY", "FTT"];

        let { borrowingGlobalAccounts, pythPrices, mints, user, userAccounts, collateralVaults } = await setupBorrowingMarketMultiple(provider, program, provider.wallet.publicKey, pyth, depositAsset, collateral);

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        for (let i = 1; i <= 5; i++) {

            const {
                assetMarket,
                requestQueue,
                eventQueue,
                baseVault,
                quoteVault,
                vaultOwner,
                marketMakerBaseTokenBalance,
                marketMakerQuoteTokenBalance
            } = await setupSerumMarket(provider, owner, usdcToken, mints[i], prices[i], marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);


            const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
                provider,
                user.publicKey,
                [user],
                user.publicKey,
                usdcToken.publicKey
            );

            const userMetadata = userAccounts.userMetadata.publicKey;

            const openOrdersUser = new Keypair();

            // Assert balances before the order placement
            let { userUsdcBalance: userUsdcBalanceBefore,
                baseVaultBalance: baseVaultBalanceBefore,
                quoteVaultBalance: quoteVaultBalanceBefore,
                // @ts-ignore
                collateralVaultBalance: collateralVaultBalanceBefore } = await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, collateralVaults[i], 0, marketMakerBtcInitialBalance - marketMakerBaseTokenBalance, marketMakerUsdcInitialBalance - marketMakerQuoteTokenBalance, depositAsset);

            await instructions_swap.serumInitAccount(
                program,
                openOrdersUser,
                assetMarket.address,
                global.DEX_PROGRAM_ID,
                user.publicKey,
                user
            );


            await instructions_swap.swapToUsdc(
                provider,
                program,
                assetMarket.address,
                openOrdersUser.publicKey,
                requestQueue,
                eventQueue,
                baseVault,
                quoteVault,
                vaultOwner,
                userUsdcAta,
                assetMarket.bidsAddress,
                assetMarket.asksAddress,
                collateral[i],
                borrowingGlobalAccounts,
                assetSwap * utils.FACTOR,
                user,
                userMetadata,
                pythPrices,
                usdcToken.publicKey
            );

            await sleep(3000);

            await assertMarketStateBalances(program, collateral[i], userAccounts, borrowingGlobalAccounts, depositAsset - assetSwap);

            // The USDC received, considering getting the best orders on the serum market
            let expectedOrderSize = orderSize * prices[i] * (1 - orderIncrements) + orderSize * prices[i] * (1 - 2 * orderIncrements);

            // @ts-ignore
            await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, collateralVaults[i], expectedOrderSize, baseVaultBalanceBefore + assetSwap, quoteVaultBalanceBefore, collateralVaultBalanceBefore - assetSwap);

            // Emptying user USDC Ata balance, so I can make the initial tests (userAtaBalance == 0)
            await emptyUserAtaBalance(provider, usdcToken, userUsdcAta, user, owner);
        }

    })

    it("tests_swap_wrong_order_accs", async () => {
        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        const btcSwap = 20;


        let { borrowingGlobalAccounts, pythPrices, assetMint: btcMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, provider.wallet.publicKey, pyth, depositBtc, "BTC");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, btcMint, btcPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        // Assert balances before the order placement
        let { userUsdcBalance: userUsdcBalanceBefore,
            baseVaultBalance: baseVaultBalanceBefore,
            quoteVaultBalance: quoteVaultBalanceBefore,
            // @ts-ignore
            collateralVaultBalance: collateralVaultBalanceBefore } = await assertSwapAccountBalances(provider, userUsdcAta, baseVault, quoteVault, borrowingGlobalAccounts.collateralVaultBtc, 0, marketMakerBtcInitialBalance - marketMakerBaseTokenBalance, marketMakerUsdcInitialBalance - marketMakerQuoteTokenBalance, depositBtc);

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            provider.wallet.publicKey,
            owner
        );

        await expect(instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            collToLamports(btcSwap, "BTC"),
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        ), 'Passing the wrong orders accounts doesn\'t get rejected').to.be.rejectedWith('0x2d');

    });

    it("tests_swap_btc_more_than_deposit", async () => {
        const btcPrice = 64622.369;

        const depositBtc = 100;

        const marketMakerBtcInitialBalance = 1000;
        const marketMakerUsdcInitialBalance = 10000000;
        const orderSize = 10;
        const orderIncrements = 0.05;

        let { borrowingGlobalAccounts, pythPrices, assetMint: btcMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, provider.wallet.publicKey, pyth, depositBtc, "BTC");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        const {
            assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            marketMakerBaseTokenBalance,
            marketMakerQuoteTokenBalance
        } = await setupSerumMarket(provider, owner, usdcToken, btcMint, btcPrice, marketMakerBtcInitialBalance, marketMakerUsdcInitialBalance, orderSize, orderIncrements);

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            user.publicKey,
            user
        );

        await expect(instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            collToLamports(depositBtc + 10, "BTC"),
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        ), `Attempting to swap more than the deposit`).to.be.rejected;

    });

    it("tests_swap_no_market", async () => {

        const depositBtc = 100;

        const btcSwap = 50;

        let { borrowingGlobalAccounts, pythPrices, assetMint: btcMint, user, userAccounts } = await setupBorrowingMarketSingle(provider, program, provider.wallet.publicKey, pyth, depositBtc, "BTC");

        const usdcToken = await serumUtils.createToken(
            provider,
            6,
            owner.publicKey,
            owner
        );

        let assetToken = new SplToken(
            provider.connection,
            btcMint,
            TOKEN_PROGRAM_ID,
            owner
        );

        const {
            market: assetMarket,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
        } = await serumUtils.createMarket(provider, owner, {
            baseToken: assetToken,
            quoteToken: usdcToken,
            baseLotSize: 1000000, // number of decimals for base token (WSOL/BTC)
            quoteLotSize: 1000, // if I adjust the decimals here, the sensitivity to decimals is higher, so I get a better precision
            feeRateBps: 0,
        });

        const userUsdcAta = await set_up.setUpAssociatedTokenAccount(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            usdcToken.publicKey
        );

        const userMetadata = userAccounts.userMetadata.publicKey;

        const openOrdersUser = new Keypair();

        await instructions_swap.serumInitAccount(
            program,
            openOrdersUser,
            assetMarket.address,
            global.DEX_PROGRAM_ID,
            user.publicKey,
            user
        );

        await expect(instructions_swap.swapToUsdc(
            provider,
            program,
            assetMarket.address,
            openOrdersUser.publicKey,
            requestQueue,
            eventQueue,
            baseVault,
            quoteVault,
            vaultOwner,
            userUsdcAta,
            assetMarket.bidsAddress,
            assetMarket.asksAddress,
            "BTC",
            borrowingGlobalAccounts,
            collToLamports(btcSwap, "BTC"),
            user,
            userMetadata,
            pythPrices,
            usdcToken.publicKey
        ), 'No swap happened passes').to.be.rejected;

    });
});

export async function setupSerumMarket(
    provider: anchor.Provider,
    owner: Account,
    usdcToken: SplToken,
    tokenMint: PublicKey,
    tokenPrice: number,
    baseTokenBalance: number,
    quoteTokenBalance: number,
    orderSize: number,
    orderIncrements: number
) {
    let assetToken = new SplToken(
        provider.connection,
        tokenMint,
        TOKEN_PROGRAM_ID,
        owner
    );

    const FACTOR = utils.FACTOR;

    const {
        market: assetMarket,
        requestQueue,
        eventQueue,
        baseVault,
        quoteVault,
        vaultOwner,
    } = await serumUtils.createMarket(provider, owner, {
        baseToken: assetToken,
        quoteToken: usdcToken,
        baseLotSize: 1000000, // number of decimals for base token (WSOL/BTC)
        quoteLotSize: 1000, // if I adjust the decimals here, the sensitivity to decimals is higher, so I get a better precision
        feeRateBps: 0,
    });

    const dexMarketMaker = await serumUtils.createMarketMaker(
        provider,
        owner,
        100 * LAMPORTS_PER_SOL,
        [
            [assetToken, new anchor.BN(baseTokenBalance * FACTOR)],
            [usdcToken, new anchor.BN(quoteTokenBalance * FACTOR)],
        ]
    );

    // how many BTC I want to sell (10) at what price (110, 120)
    const asks = serumUtils.makeOrders([
        [tokenPrice * (1 + orderIncrements), orderSize],
        [tokenPrice * (1 + 2 * orderIncrements), orderSize],
    ]);

    // how many BTC I want to buy (10) at what price (100, 90)
    const bids = serumUtils.makeOrders([
        [tokenPrice * (1 - orderIncrements), orderSize],
        [tokenPrice * (1 - 2 * orderIncrements), orderSize],
    ]);

    await serumUtils.placeOrders(
        provider,
        dexMarketMaker,
        assetMarket,
        bids,
        asks,
    );

    const baseTokenAccount =
        dexMarketMaker.tokenAccounts[assetMarket.baseMintAddress.toBase58()];
    const quoteTokenAccount =
        dexMarketMaker.tokenAccounts[assetMarket.quoteMintAddress.toBase58()];

    let marketMakerUsdcBalanceAfter = await (
        await provider.connection.getTokenAccountBalance(quoteTokenAccount)
    ).value.uiAmount;
    let marketMakerBtcBalanceAfter = await (
        await provider.connection.getTokenAccountBalance(baseTokenAccount)
    ).value.uiAmount;

    console.log("MARKET MAKER BTC BALANCE", marketMakerBtcBalanceAfter);
    console.log("MARKET MAKER USDC BALANCE", marketMakerUsdcBalanceAfter);


    return {
        assetMarket,
        requestQueue,
        eventQueue,
        baseVault,
        quoteVault,
        vaultOwner,
        usdcToken,
        marketMakerBaseTokenBalance: marketMakerBtcBalanceAfter,
        marketMakerQuoteTokenBalance: marketMakerUsdcBalanceAfter,
    };
}

export async function setupBorrowingMarketSingle(provider: anchor.Provider, program: anchor.Program, initialMarketOwner: PublicKey, pyth: PythUtils, depositAsset: number, asset: CollateralToken) {
    const pythPrices = await set_up.setUpPythPrices(
        provider,
        pyth,
    );

    let { user, userAccounts, borrowingGlobalAccounts } =
        await operations_borrowing.setUpMarketWithEmptyUser(
            provider,
            program,
            initialMarketOwner
        );

    let mintAsset = 700;

    let assetMint, collateralVault, userAta;
    switch (asset) {
        case "BTC":
            {
                assetMint = borrowingGlobalAccounts.btcMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultBtc;
                userAta = userAccounts.btcAta;
            }
            break;
        case "ETH":
            {
                assetMint = borrowingGlobalAccounts.ethMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultEth;
                userAta = userAccounts.ethAta;
            }
            break;
        case "SRM":
            {
                assetMint = borrowingGlobalAccounts.srmMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultSrm;
                userAta = userAccounts.srmAta;

            }
            break;
        case "RAY":
            {
                assetMint = borrowingGlobalAccounts.rayMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultRay;
                userAta = userAccounts.rayAta;
            }
            break;
        case "FTT":
            {
                assetMint = borrowingGlobalAccounts.fttMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultFtt;
                userAta = userAccounts.fttAta;
            }
            break;
        case "SOL":
            {
                assetMint = borrowingGlobalAccounts.btcMint;
                collateralVault = borrowingGlobalAccounts.collateralVaultBtc;
                userAta = userAccounts.btcAta;
            }
            break;
    }

    await operations_borrowing.mintToAta(
        provider,
        borrowingGlobalAccounts,
        userAccounts,
        asset,
        collToLamports(mintAsset, asset)
    );


    // deposit 500 BTC
    await instructions_borrow.depositCollateral(
        program,
        user.publicKey,
        userAccounts.userMetadata.publicKey,
        collateralVault,
        userAta,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.borrowingVaults.publicKey,
        collToLamports(depositAsset, asset),
        [user],
        asset
    );

    console.log(`Deposited ${asset}`);



    return { borrowingGlobalAccounts, pythPrices, assetMint, user, userAccounts };

}

export async function setupBorrowingMarketMultiple(provider: anchor.Provider, program: anchor.Program, initialMarketOwner: PublicKey, pyth: PythUtils, depositAsset: number, collateral: CollateralToken[]) {
    const pythPrices = await set_up.setUpPythPrices(
        provider,
        pyth,
    );

    let { user, userAccounts, borrowingGlobalAccounts } =
        await operations_borrowing.setUpMarketWithEmptyUser(
            provider,
            program,
            initialMarketOwner
        );

    let mintAsset = 700;

    let collateralVault, userAta;

    // TODO: First mint should be mintWsol, adding mintBtc as a placeholder
    let mints = [borrowingGlobalAccounts.btcMint, borrowingGlobalAccounts.ethMint, borrowingGlobalAccounts.btcMint, borrowingGlobalAccounts.srmMint, borrowingGlobalAccounts.rayMint, borrowingGlobalAccounts.fttMint];
    let collateralVaults = [borrowingGlobalAccounts.collateralVaultSol, borrowingGlobalAccounts.collateralVaultEth, borrowingGlobalAccounts.collateralVaultBtc, borrowingGlobalAccounts.collateralVaultSrm, borrowingGlobalAccounts.collateralVaultRay, borrowingGlobalAccounts.collateralVaultFtt];
    for (let i = 1; i < collateral.length; i++) {
        await operations_borrowing.mintToAta(
            provider,
            borrowingGlobalAccounts,
            userAccounts,
            collateral[i],
            collToLamports(mintAsset, collateral[i])
        );

        switch (collateral[i]) {
            case "SOL": {
                //TODO: add wsol
                collateralVault = borrowingGlobalAccounts.collateralVaultBtc;
                userAta = userAccounts.btcAta;
            } break;
            case "BTC": {
                collateralVault = borrowingGlobalAccounts.collateralVaultBtc;
                userAta = userAccounts.btcAta
            } break;
            case "ETH": {
                collateralVault = borrowingGlobalAccounts.collateralVaultEth;
                userAta = userAccounts.ethAta

            } break;
            case "SRM": {
                collateralVault = borrowingGlobalAccounts.collateralVaultSrm;
                userAta = userAccounts.srmAta

            } break;
            case "RAY": {
                collateralVault = borrowingGlobalAccounts.collateralVaultRay;
                userAta = userAccounts.rayAta
            } break;
            case "FTT": {
                collateralVault = borrowingGlobalAccounts.collateralVaultFtt;
                userAta = userAccounts.fttAta

            } break;
        }

        // deposit collateral
        await instructions_borrow.depositCollateral(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            collateralVault,
            userAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            collToLamports(depositAsset, collateral[i]),
            [user],
            collateral[i]
        );

        console.log(`Deposited ${collateral[i]}`);
    }


    return { borrowingGlobalAccounts, pythPrices, mints, user, userAccounts, collateralVaults };

}

export async function emptyUserAtaBalance(provider: anchor.Provider, usdcToken: SplToken, userUsdcAta: PublicKey, fromAccount: Keypair, toAccount: Account) {
    // @ts-ignore
    let tokenBalance = Number.parseInt(await (await provider.connection.getTokenAccountBalance(userUsdcAta)).value.amount);
    console.log("Usdc Token Balance Before Transfer", tokenBalance);
    // Create associated token accounts for my token if they don't exist yet
    var fromTokenAccount = await usdcToken.getOrCreateAssociatedAccountInfo(
        fromAccount.publicKey
    )
    var toTokenAccount = await usdcToken.getOrCreateAssociatedAccountInfo(
        toAccount.publicKey
    )
    // Add token transfer instructions to transaction
    var transaction = new web3.Transaction()
        .add(
            splToken.Token.createTransferInstruction(
                splToken.TOKEN_PROGRAM_ID,
                fromTokenAccount.address,
                toTokenAccount.address,
                fromAccount.publicKey,
                [],
                tokenBalance
            )
        );
    // Sign transaction, broadcast, and confirm
    var signature = await web3.sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [fromAccount]
    );
    console.log("Usdc Token transfer signature", signature);

    //@ts-ignore
    let tokenBalanceAfter = Number.parseFloat(await (await provider.connection.getTokenAccountBalance(userUsdcAta)).value.uiAmountString);
    console.log("Usdc Token Balance after transfer (should be 0)", tokenBalanceAfter);
}

export async function assertSwapAccountBalances(provider: anchor.Provider, userUsdcAta: PublicKey, baseVault: PublicKey, quoteVault: PublicKey, collateralVault: PublicKey, expectedUserUsdcBalance: number, expectedBaseVaultBalance: number, expectedQuoteVaultBalance: number, expectedCollateralVaultBalance: number) {
    let userUsdcBalance = await (
        await provider.connection.getTokenAccountBalance(userUsdcAta)
    ).value.uiAmount;

    let quoteVaultBalance = await (
        await provider.connection.getTokenAccountBalance(quoteVault)
    ).value.uiAmount;

    let baseVaultBalance = await (
        await provider.connection.getTokenAccountBalance(baseVault)
    ).value.uiAmount;

    let collateralVaultBalance = await (
        await provider.connection.getTokenAccountBalance(
            collateralVault
        )
    ).value.uiAmount;

    if (expectedUserUsdcBalance !== 0) {
        // assert that the userUsdcBalance and the quoteVaultBalance left is equal to the initial quoteVaultBalance amount
        // Small precision loss when I created the market
        assert.ok(
            // @ts-ignore
            quoteVaultBalance + userUsdcBalance -
            expectedQuoteVaultBalance < 0.00000001
        );
        console.log("USER USDC BALANCE IS", userUsdcBalance);
        console.log("QUOTE VAULT BALANCE IS", quoteVaultBalance);

        // @ts-ignore
        assert.ok(userUsdcBalance > 0);

        console.log("expectedOrderSize", expectedUserUsdcBalance)

        // @ts-ignore
        console.log("Takers fee", 1 - userUsdcBalance / expectedUserUsdcBalance);
        // 0.22% fees for takers
        // @ts-ignore
        assert.ok(1 - userUsdcBalance / expectedUserUsdcBalance < 0.0022);

    } else {
        console.log("USER USDC BALANCE IS", userUsdcBalance);
        assert.strictEqual(userUsdcBalance, expectedUserUsdcBalance);

        console.log("QUOTE VAULT BALANCE IS", quoteVaultBalance);
        // Small precision loss when I created the market
        assert.ok(
            //@ts-ignore
            quoteVaultBalance - expectedQuoteVaultBalance < 0.00000001
        );
    }

    console.log("BASE VAULT BALANCE IS", baseVaultBalance);
    assert.strictEqual(
        baseVaultBalance,
        expectedBaseVaultBalance
    );

    console.log("COLLATERAL VAULT BALANCE IS", collateralVaultBalance);
    assert.strictEqual(collateralVaultBalance, expectedCollateralVaultBalance);

    return { userUsdcBalance, baseVaultBalance, quoteVaultBalance, collateralVaultBalance }
}

export async function assertMarketStateBalances(program: anchor.Program, collateral: CollateralToken, userAccounts: set_up.BorrowingUserAccounts, borrowingGlobalAccounts: set_up.BorrowingGlobalAccounts, stateValue: number) {
    let userMetadataAfter = await getUserMetadata(program, userAccounts.userMetadata.publicKey);
    assert.strictEqual(userMetadataAfter.borrowedStablecoin, 0);
    const { inactiveCollateral } = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

    let userMetadataTokenBalance, borrowingMarketStateTokenBalance;

    switch (collateral) {
        case "ETH":
            {
                userMetadataTokenBalance = userMetadataAfter.inactiveCollateral.eth;
                borrowingMarketStateTokenBalance = inactiveCollateral.eth;
            }
            break;
        case "BTC":
            {
                userMetadataTokenBalance = userMetadataAfter.inactiveCollateral.btc;
                borrowingMarketStateTokenBalance = inactiveCollateral.btc;
            }
            break;
        case "SRM":
            {
                userMetadataTokenBalance = userMetadataAfter.inactiveCollateral.srm;
                borrowingMarketStateTokenBalance = inactiveCollateral.srm;
            }
            break;
        case "RAY":
            {
                userMetadataTokenBalance = userMetadataAfter.inactiveCollateral.ray;
                borrowingMarketStateTokenBalance = inactiveCollateral.ray;
            }
            break;
        case "FTT":
            {
                userMetadataTokenBalance = userMetadataAfter.inactiveCollateral.ftt;
                borrowingMarketStateTokenBalance = inactiveCollateral.ftt;
            }
            break;
    }
    assert.strictEqual(userMetadataTokenBalance, collToLamports(stateValue, collateral));
    assert.strictEqual(borrowingMarketStateTokenBalance, collToLamports(stateValue, collateral));
}