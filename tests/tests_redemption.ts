import * as anchor from "@project-serum/anchor";
import * as operations_borrowing from './operations_borrowing';
import { newLoanee } from './operations_borrowing';
import * as operations_redemption from './operations_redemption';
import * as set_up from "../src/set_up";
import { BorrowingGlobalAccounts, BorrowingUserState, StakingPoolAccounts, setUpProgram } from "../src/set_up";

import { CollateralToken } from "./types";
import { assertBorrowerBalance, assertBorrowerCollateral, assertBurningVaultBalance, assertGlobalCollateral, assertGlobalDebt, assertRedemptionsQueueOrderFilled, assertRedemptionsQueueSize } from "./test_assertions";
import { PublicKey, TransactionSignature } from "@solana/web3.js";
import { sleep } from "@project-serum/common";
import { getRedemptionsQueueData } from "./data_provider";

import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'

chai.use(chaiAsPromised)

const REDEMPTION_CLEAR_WAIT_TIME: number = 5000;
const REDEMPTION_ORDERS_QUEUE_SIZE: number = 15;
const REDEMPTION_CANDIDATE_QUEUE_SIZE: number = 32;
export const FILL_INST_METADATA_ACCS_SIZE: number = 3;
export const CLEAR_INST_METADATA_ACCS_SIZE: number = 6;

describe('tests_redemption', () => {

    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('tests_redemption_add_fill_and_clear_simple', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        // Price is 10K each
        const pythPrices = await set_up.setUpPrices(
            provider,
            pyth,
            {
                solPrice: 10000.0,
                ethPrice: 10000.0,
                btcPrice: 10000.0,
                srmPrice: 10000.0,
                fttPrice: 10000.0,
                rayPrice: 10000.0,
            }
        );

        const redeemAmount = 8000;

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)
        const fillUser = await operations_redemption.newFillUser(env, borrowingGlobalAccounts)
        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        // To basically have a low impact on the redemption fee
        // due to redemption amount being much lower than total supply
        let whaleBorrow = 10000000.0;
        let whaleDebt = 10050000.0;
        const whale = await operations_borrowing.newLoanee(
            env,
            borrowingGlobalAccounts,
            stakingPoolAccounts,
            pythPrices,
            whaleBorrow,
            new Map<CollateralToken, number>([["ETH", 1000000],])
        );

        // Borrowing              =    10_000
        // Depositing 100 * 10000 = 1_000_000
        const loanee = await operations_borrowing.newLoanee(
            env,
            borrowingGlobalAccounts,
            stakingPoolAccounts,
            pythPrices,
            10000.0,
            new Map<CollateralToken, number>([["ETH", 100],])
        );

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts,
            fillUser, 0, [
            loanee.borrowerAccounts.userMetadata.publicKey,
        ]
        );

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts, [
            {
                loaneeMetadata: loanee.borrowerAccounts.userMetadata.publicKey,
                fillerMetadata: fillUser.borrowerAccounts.userMetadata.publicKey,
            }
        ]);

        await waitAndClear(provider, program, borrowingGlobalAccounts,
            clearUser, redemptionUser, 0, [
            loanee.borrowerAccounts.userMetadata.publicKey,
            fillUser.borrowerAccounts.userMetadata.publicKey,
        ]
        );

        // stablecoin debt reduced by redemption amount
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, whaleDebt + 2050);
        // assert borrower balance
        await assertBorrowerBalance(provider, program, loanee.borrower, loanee.borrowerAccounts, borrowingGlobalAccounts, 2050, loanee.borrowerInitialBalance, 10000);
        await assertBorrowerCollateral(provider, program, loanee.borrower, loanee.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 99.2],
        ]));
        // assert redeemer balance
        await assertBorrowerCollateral(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.79576],
        ]), "inactive");
        // assert filler balance
        await assertBorrowerCollateral(provider, program, fillUser.borrower, fillUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.0004],
        ]), "inactive");
        // assert clearer balance
        await assertBorrowerCollateral(provider, program, clearUser.borrower, clearUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.0004],
        ]), "inactive");
    });


    it('tests_redemption_add_fill_and_clear_multiple_collateral', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPrices(
            provider,
            pyth,
            {
                solPrice: 10000.0,
                ethPrice: 10000.0,
                btcPrice: 10000.0,
                srmPrice: 10000.0,
                fttPrice: 10000.0,
                rayPrice: 10000.0,
            }
        );

        // To basically have a low impact on the redemption fee
        // due to redemption amount being much lower than total supply
        let whaleBorrow = 10000000.0;
        let whaleCollEth = 1000000;
        let whaleDebt = 10050000.0;
        const whale = await operations_borrowing.newLoanee(
            env,
            borrowingGlobalAccounts,
            stakingPoolAccounts,
            pythPrices,
            whaleBorrow,
            new Map<CollateralToken, number>([["ETH", whaleCollEth],])
        );

        const redeemAmount = 8000;

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)
        const fillUser = await operations_redemption.newFillUser(env, borrowingGlobalAccounts)
        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        const loanee1 = await operations_borrowing.newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 10000, new Map<CollateralToken, number>([
            ["SOL", 12],
            ["BTC", 123],
        ]));
        const loanee2 = await operations_borrowing.newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 10000, new Map<CollateralToken, number>([
            ["SRM", 1],
            ["ETH", 10],
        ]));

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, 0, [
            loanee1.borrowerAccounts.userMetadata.publicKey,
            loanee2.borrowerAccounts.userMetadata.publicKey,
        ]);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts, [
            {
                loaneeMetadata: loanee2.borrowerAccounts.userMetadata.publicKey,
                fillerMetadata: fillUser.borrowerAccounts.userMetadata.publicKey
            },
            {
                loaneeMetadata: loanee1.borrowerAccounts.userMetadata.publicKey,
                fillerMetadata: fillUser.borrowerAccounts.userMetadata.publicKey
            },
        ]);

        await waitAndClear(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser, 0, [
            loanee1.borrowerAccounts.userMetadata.publicKey,
            loanee2.borrowerAccounts.userMetadata.publicKey,
            fillUser.borrowerAccounts.userMetadata.publicKey,
        ]);

        // global collateral unchanged
        // Some collateral became inactive
        console.log("BB");
        await assertGlobalCollateral(program, provider, borrowingGlobalAccounts.borrowingMarketState.publicKey, borrowingGlobalAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([
                ["SOL", 12],
                ["BTC", 123],
                ["SRM", 0.927273],
                ["ETH", 9.272728 + whaleCollEth],
            ]),
            new Map<CollateralToken, number>([
                ["SRM", 1 - 0.927273],
                ["ETH", 0.7272720000473782],
            ])
        );
        // 0.7272720000473782 + 9.272728 = 10.000000000047379
        // stablecoin debt reduced by redemption amount
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, whaleDebt + 12100);
        // assert borrower balances
        console.log("A");
        await assertBorrowerBalance(provider, program, loanee1.borrower, loanee1.borrowerAccounts, borrowingGlobalAccounts, 10050, loanee1.borrowerInitialBalance - 12, 10000);
        console.log("B");
        await assertBorrowerCollateral(provider, program, loanee1.borrower, loanee1.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 12],
            ["BTC", 123],
        ]));
        console.log("C");
        await assertBorrowerBalance(provider, program, loanee2.borrower, loanee2.borrowerAccounts, borrowingGlobalAccounts, 2050, loanee2.borrowerInitialBalance, 10000);
        console.log("D");
        await assertBorrowerCollateral(provider, program, loanee2.borrower, loanee2.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SRM", 0.927273],
            ["ETH", 9.272728],
        ]), "deposited");
        console.log("E");
        // assert redeemer balance
        await assertBorrowerCollateral(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.723419],
            ["SRM", 0.072343],
        ]), "inactive");
        // assert filler balance
        await assertBorrowerCollateral(provider, program, fillUser.borrower, fillUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.000363],
            ["SRM", 0.000036],
        ]), "inactive");
        // assert clearer balance
        await assertBorrowerCollateral(provider, program, clearUser.borrower, clearUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 0.000363],
            ["SRM", 0.000036],
        ]), "inactive");
    });

    it('tests_redemption_add_fill_and_clear_paginated_order', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        // To basically have a low impact on the redemption fee
        // due to redemption amount being much lower than total supply
        let whaleBorrow = 10000000.0;
        let whaleDebt = 10050000.0;
        const whale = await operations_borrowing.newLoanee(
            env,
            borrowingGlobalAccounts,
            stakingPoolAccounts,
            pythPrices,
            whaleBorrow,
            new Map<CollateralToken, number>([["ETH", 1000000],])
        );

        const loaneeBorrowAmount = 3000;
        const loaneeFee = loaneeBorrowAmount * 0.005;
        const loaneeDebt = loaneeBorrowAmount + loaneeFee;
        const numberOfLoanees = (REDEMPTION_CANDIDATE_QUEUE_SIZE * 2) + 1; // extra borrower to not redeem more than borrowed
        const redeemAmount = (numberOfLoanees - 1) * loaneeDebt;

        const loanees = await newLoaneesEqualCollateralRatio(env, borrowingGlobalAccounts, stakingPoolAccounts, numberOfLoanees, loaneeBorrowAmount, pythPrices);

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)
        const fillUser = await operations_redemption.newFillUser(env, borrowingGlobalAccounts)
        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        // fill first page
        for (let i = 0, batch = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i += batch) {
            const startUser = i;
            const endUser = Math.min(REDEMPTION_CANDIDATE_QUEUE_SIZE, i + FILL_INST_METADATA_ACCS_SIZE);
            batch = endUser - startUser;
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, 0,
                takeRangeOfLoanees(loanees, startUser, endUser)
            )
        }

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts,
            createExpectedFillRange(loanees, fillUser, 0, REDEMPTION_CANDIDATE_QUEUE_SIZE));

        // wait for order to be clearable
        await waitToClear();

        // clear first page
        for (let i = 0, batch = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i += batch) {
            // subtract 1 to allow space for fill user
            batch = Math.min(CLEAR_INST_METADATA_ACCS_SIZE - 1, REDEMPTION_CANDIDATE_QUEUE_SIZE - i);
            await clearWithRetry(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser, 0, [
                ...takeRangeOfLoanees(loanees, i, i + batch),
                fillUser.borrowerAccounts.userMetadata.publicKey,
            ]);
        }

        // still 1 active order, half redeemed
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault,
            redeemAmount / 2);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        // fill second page
        for (let i = REDEMPTION_CANDIDATE_QUEUE_SIZE, batch = 0; i < numberOfLoanees; i += batch) {
            const startUser = i;
            const endUser = Math.min(numberOfLoanees, i + FILL_INST_METADATA_ACCS_SIZE);
            batch = endUser - startUser;
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, 0,
                takeRangeOfLoanees(loanees, startUser, endUser)
            )
        }

        // wait 5s for order to be clearable
        await waitToClear();

        // clear second page
        for (let i = REDEMPTION_CANDIDATE_QUEUE_SIZE, batch = 0; i < numberOfLoanees; i += batch) {
            let numOrders = await getRedemptionsQueueSize(program, borrowingGlobalAccounts);
            if (numOrders === 0) {
                break;
            }
            // subtract 1 to allow space for fill user
            batch = Math.min(CLEAR_INST_METADATA_ACCS_SIZE - 1, numberOfLoanees - i);
            const borrowerAndFillerMetadatas = [
                ...takeRangeOfLoanees(loanees, i, i + batch),
                fillUser.borrowerAccounts.userMetadata.publicKey,
            ];
            await clearWithRetry(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser, 0, borrowerAndFillerMetadatas);
        }

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 0);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, 0);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, loaneeDebt + whaleDebt);

        // assert borrower balances
        for (let i = 0; i < numberOfLoanees - 1; i++) {
            await assertBorrowerBalance(provider, program, loanees[i].borrower, loanees[i].borrowerAccounts, borrowingGlobalAccounts, 0, loanees[i].borrowerInitialBalance - 0.5, loaneeBorrowAmount);
            await assertBorrowerCollateral(provider, program, loanees[i].borrower, loanees[i].borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
                ["SOL", 0.478321991],
                ["ETH", 0.956644],
                ["BTC", 0.956644],
                ["SRM", 0.956644],
                ["RAY", 0.956644],
                ["FTT", 0.956644],
            ]), "inactive");
        }
        // extra borrower still has initial collateral, debt and balance
        await assertBorrowerBalance(provider, program, loanees[numberOfLoanees - 1].borrower, loanees[numberOfLoanees - 1].borrowerAccounts, borrowingGlobalAccounts, loaneeDebt, loanees[numberOfLoanees - 1].borrowerInitialBalance - 0.5, loaneeBorrowAmount);
        await assertBorrowerCollateral(provider, program, loanees[numberOfLoanees - 1].borrower, loanees[numberOfLoanees - 1].borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.5],
            ["ETH", 1],
            ["BTC", 1],
            ["SRM", 1],
            ["RAY", 1],
            ["FTT", 1],
        ]));
        // assert redeemer balance
        await assertBorrowerCollateral(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 1.367414144],
            ["ETH", 2.734976],
            ["BTC", 2.734976],
            ["SRM", 2.734976],
            ["RAY", 2.734976],
            ["FTT", 2.734976],
        ]), "inactive");
        // assert filler balance
        await assertBorrowerCollateral(provider, program, fillUser.borrower, fillUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.000693696],
            ["ETH", 0.001344],
            ["BTC", 0.001344],
            ["SRM", 0.001344],
            ["RAY", 0.001344],
            ["FTT", 0.001344],
        ]), "inactive");
        // assert clearer balance
        await assertBorrowerCollateral(provider, program, clearUser.borrower, clearUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.000693696],
            ["ETH", 0.001344],
            ["BTC", 0.001344],
            ["SRM", 0.001344],
            ["RAY", 0.001344],
            ["FTT", 0.001344],
        ]), "inactive");
    });

    it('tests_redemption_test_redemption_orders_queue', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        // Price is 10K each
        const pythPrices = await set_up.setUpPrices(
            provider,
            pyth,
            {
                solPrice: 10000.0,
                ethPrice: 10000.0,
                btcPrice: 10000.0,
                srmPrice: 10000.0,
                fttPrice: 10000.0,
                rayPrice: 10000.0,
            }
        );

        // To basically have a low impact on the redemption fee
        // due to redemption amount being much lower than total supply
        let whaleBorrow = 1000000000.0;
        let whaleDebt = 1005000000.0;
        const whale = await operations_borrowing.newLoanee(
            env,
            borrowingGlobalAccounts,
            stakingPoolAccounts,
            pythPrices,
            whaleBorrow,
            new Map<CollateralToken, number>([["ETH", 10000000],])
        );

        const user1RedeemAmount = (2100 * REDEMPTION_CANDIDATE_QUEUE_SIZE) * 1.005;
        const user2RedeemAmount = (2100 * REDEMPTION_CANDIDATE_QUEUE_SIZE) * 1.005;
        const user3RedeemAmount = (2100 * REDEMPTION_CANDIDATE_QUEUE_SIZE) * 1.005;
        const totalRedeemAmount = user1RedeemAmount + user2RedeemAmount + user3RedeemAmount;

        const redemptionUser1 = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, user1RedeemAmount, 10)
        const redemptionUser2 = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, user2RedeemAmount, 10)
        const redemptionUser3 = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, user3RedeemAmount, 10)
        const fillUser = await operations_redemption.newFillUser(env, borrowingGlobalAccounts)
        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        const borrowStablecoinAmount = ((totalRedeemAmount / 100.5) * 100) / 2; // 2 users, deduct the fees

        console.log("borrowStablecoinAmount", borrowStablecoinAmount);
        console.log("user1RedeemAmount", user1RedeemAmount);
        console.log("user2RedeemAmount", user2RedeemAmount);
        console.log("user3RedeemAmount", user3RedeemAmount);

        const loanee1 = await operations_borrowing.newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, borrowStablecoinAmount, new Map<CollateralToken, number>([
            ["SOL", 12],
            ["BTC", 5000],
        ]));
        const loanee2 = await operations_borrowing.newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, borrowStablecoinAmount, new Map<CollateralToken, number>([
            ["SRM", 1],
            ["ETH", 100],
        ]));

        // fill redemption orders queue
        for (let i = 0; i < REDEMPTION_ORDERS_QUEUE_SIZE; i++) {
            await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser1, pythPrices,
                user1RedeemAmount / REDEMPTION_ORDERS_QUEUE_SIZE);
        }

        // redemption queue full
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, REDEMPTION_ORDERS_QUEUE_SIZE);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, user1RedeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser1.borrower, redemptionUser1.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser1.borrowerInitialBalance, 0);

        // adding more orders is forbidden
        console.log("ERROR BELOW IS EXPECTED 1...");
        await expect(
            operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts,
                redemptionUser2, pythPrices, user2RedeemAmount / REDEMPTION_ORDERS_QUEUE_SIZE
            ), `Attempting to add order to full queue was not rejected`)
            .to.be.rejected;
        console.log(`ERROR ABOVE WAS EXPECTED!`)

        // fill and clear all orders - add new order after each clear
        for (let i = 0; i < REDEMPTION_ORDERS_QUEUE_SIZE; i++) {
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, i,
                takeRangeOfLoanees([loanee1, loanee2], 0, 2)
            );
            await waitAndClear(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser1, i, [
                ...takeRangeOfLoanees([loanee1, loanee2], 0, 2),
                fillUser.borrowerAccounts.userMetadata.publicKey,
            ]);
            await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser2, pythPrices, user2RedeemAmount / REDEMPTION_ORDERS_QUEUE_SIZE);
        }

        // redemption queue still full
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, REDEMPTION_ORDERS_QUEUE_SIZE);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, user2RedeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser2.borrower, redemptionUser2.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser2.borrowerInitialBalance, 0);

        // fill and clear all orders
        for (let i = 0; i < REDEMPTION_ORDERS_QUEUE_SIZE; i++) {
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, i,
                takeRangeOfLoanees([loanee1, loanee2], 0, 2)
            )
            await waitAndClear(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser2, i, [
                ...takeRangeOfLoanees([loanee1, loanee2], 0, 2),
                fillUser.borrowerAccounts.userMetadata.publicKey,
            ]);
        }

        // redemption queue empty
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 0);

        // add another order
        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser3, pythPrices, user3RedeemAmount / 2);

        // redemption queue has 1 order
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);

        // fill order
        await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, 0,
            takeRangeOfLoanees([loanee1, loanee2], 1, 2)
        )

        // add another order while filling
        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser3, pythPrices, user3RedeemAmount / 2);

        // redemption queue has 2 orders
        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 2);

        // clear the active order
        await waitAndClear(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser3, 0, [
            ...takeRangeOfLoanees([loanee1, loanee2], 1, 2),
            fillUser.borrowerAccounts.userMetadata.publicKey,
        ]);

        // fill and clear the final order
        await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUser, 1,
            takeRangeOfLoanees([loanee1, loanee2], 1, 2)
        )

        await waitAndClear(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser3, 1, [
            ...takeRangeOfLoanees([loanee1, loanee2], 1, 2),
            fillUser.borrowerAccounts.userMetadata.publicKey,
        ]);

        // stablecoin debt reduced by redemption amount
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, whaleDebt + 67536);
        // assert borrower balances
        await assertBorrowerBalance(provider, program, loanee1.borrower, loanee1.borrowerAccounts, borrowingGlobalAccounts, 67536, loanee1.borrowerInitialBalance - 12, borrowStablecoinAmount);
        await assertBorrowerCollateral(provider, program, loanee1.borrower, loanee1.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 11.99191509],
            ["BTC", 4996.631285],
        ]));
        await assertBorrowerBalance(provider, program, loanee2.borrower, loanee2.borrowerAccounts, borrowingGlobalAccounts, 0, loanee2.borrowerInitialBalance, borrowStablecoinAmount);
        await assertBorrowerCollateral(provider, program, loanee2.borrower, loanee2.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 89.969905],
            ["SRM", 0.899718],
        ]), "inactive");
        // assert redeemer balances
        await assertBorrowerCollateral(provider, program, redemptionUser1.borrower, redemptionUser1.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 6.653325],
            ["SRM", 0.06654],
        ]), "inactive");
        await assertBorrowerCollateral(provider, program, redemptionUser2.borrower, redemptionUser2.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.008044508],
            ["ETH", 3.326663],
            ["BTC", 3.351885],
            ["SRM", 0.03327],
        ]), "inactive");
        await assertBorrowerCollateral(provider, program, redemptionUser3.borrower, redemptionUser3.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            // ["SOL", 0.015953159],
            // ["BTC", 6.64715],
        ]), "inactive");
        // assert filler balance
        await assertBorrowerCollateral(provider, program, fillUser.borrower, fillUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.000004035],
            ["ETH", 0.004995],
            ["BTC", 0.00168],
            ["SRM", 0.000045],
        ]), "inactive");
        // assert clearer balance
        await assertBorrowerCollateral(provider, program, clearUser.borrower, clearUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 0.000004035],
            ["ETH", 0.004995],
            ["BTC", 0.00168],
            ["SRM", 0.000045],
        ]), "inactive");
    });

    it('tests_redemption_add_and_fill_with_duplicate_candidates', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const redeemAmount = 8000;
        const numberOfFillUsers = 3;
        const numberOfLoanees = 6;

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)
        const [fillUser1, fillUser2, fillUser3,] = await operations_redemption.newFillUsers(env, borrowingGlobalAccounts, numberOfFillUsers);

        const [
            loanee1,
            loanee2,
            loanee3,
            loanee4,
            loanee5,
            loanee6,
        ] = await newLoaneesDescendingCollateralRatio(env, borrowingGlobalAccounts, stakingPoolAccounts, numberOfLoanees, pythPrices);

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        // e.g. [1, 1, 1, 1, 1, 1, 1, 1, 1]
        let sameUserRepeated = [...Array(FILL_INST_METADATA_ACCS_SIZE)]
            .map(_ => loanee1.borrowerAccounts.userMetadata.publicKey);

        // e.g. [2, 3, 2, 3, 2, 3, 2, 3, _]
        let alternatingUsersRepeated = [...Array(Math.max(2, Math.floor(FILL_INST_METADATA_ACCS_SIZE / 2)))]
            .map(_ => {
                return [
                    loanee2.borrowerAccounts.userMetadata.publicKey,
                    loanee3.borrowerAccounts.userMetadata.publicKey
                ]
            }).reduce((previousValue, currentValue) => {
                return [...previousValue, ...currentValue];
            }, []);
        if (alternatingUsersRepeated.length > FILL_INST_METADATA_ACCS_SIZE) {
            alternatingUsersRepeated = alternatingUsersRepeated.slice(0, FILL_INST_METADATA_ACCS_SIZE);
        }

        // e.g. [4, 4, 4, 5, 5, 5, 6, 6, 6]
        let sameUsersRepeated =
            [...[...Array(Math.max(2, Math.floor(FILL_INST_METADATA_ACCS_SIZE / 3)))]
                .map(_ => loanee4.borrowerAccounts.userMetadata.publicKey)
                , ...[...Array(Math.max(2, Math.floor(FILL_INST_METADATA_ACCS_SIZE / 3)))]
                    .map(_ => loanee5.borrowerAccounts.userMetadata.publicKey)
                , ...[...Array(Math.max(2, Math.floor(FILL_INST_METADATA_ACCS_SIZE / 3)))]
                    .map(_ => loanee6.borrowerAccounts.userMetadata.publicKey)
            ]
        if (sameUserRepeated.length > FILL_INST_METADATA_ACCS_SIZE) {
            sameUserRepeated = sameUserRepeated.slice(0, FILL_INST_METADATA_ACCS_SIZE);
        }
        console.log(`same userrr reddddd - ${alternatingUsersRepeated}`)

        console.log("1) ERROR BELOW IS EXPECTED...");
        await expect(operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts,
            fillUser1, 0,
            sameUserRepeated
        )).to.be.rejectedWith("Duplicate account in fill order");
        console.log(`ERROR ABOVE WAS EXPECTED!`)

        console.log("2) ERROR BELOW IS EXPECTED...");
        await expect(operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts,
            fillUser2, 0,
            alternatingUsersRepeated
        )).to.be.rejectedWith("Duplicate account in fill order");
        console.log(`ERROR ABOVE WAS EXPECTED!`)

        console.log("3) ERROR BELOW IS EXPECTED...");
        await expect(operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts,
            fillUser3, 0,
            sameUsersRepeated
        )).to.be.rejectedWith("Duplicate account in fill order");
        console.log(`ERROR ABOVE WAS EXPECTED!`)

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
    });

    it('tests_redemption_add_fill_and_clear_with_duplicate_candidates', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const redeemAmount = 8000;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)
        const fillUser = await operations_redemption.newFillUser(env, borrowingGlobalAccounts)
        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        const loanee = await operations_borrowing.newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 10000, new Map<CollateralToken, number>([
            ["ETH", 100],
        ]));
        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts,
            fillUser, 0, [
            loanee.borrowerAccounts.userMetadata.publicKey,
        ]);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts, [
            {
                loaneeMetadata: loanee.borrowerAccounts.userMetadata.publicKey,
                fillerMetadata: fillUser.borrowerAccounts.userMetadata.publicKey,
            }
        ]);

        // e.g [loanee, filler1, filler1, filler1, filler1]
        let fillerRepeated = [...Array(CLEAR_INST_METADATA_ACCS_SIZE - 1).keys()].map(_ => {
            return fillUser.borrowerAccounts.userMetadata.publicKey
        }).reduce((previousValue, currentValue) => [...previousValue, currentValue],
            [loanee.borrowerAccounts.userMetadata.publicKey]
        );
        await expect(waitAndClear(provider, program, borrowingGlobalAccounts,
            clearUser, redemptionUser, 0, fillerRepeated
        )).to.be.rejected;
        console.log(`ABOVE ERRORS WERE EXPECTED!`)

        // stablecoin debt reduced by redemption amount
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, 10050);
        // assert borrower balance unchanged
        await assertBorrowerBalance(provider, program, loanee.borrower, loanee.borrowerAccounts, borrowingGlobalAccounts, 10050, loanee.borrowerInitialBalance, 10000);
        await assertBorrowerCollateral(provider, program, loanee.borrower, loanee.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 100],
        ]));
        // assert redeemer balance
        await assertBorrowerCollateral(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([]));
        // assert filler balance
        await assertBorrowerCollateral(provider, program, fillUser.borrower, fillUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([]));
        // assert clearer balance
        await assertBorrowerCollateral(provider, program, clearUser.borrower, clearUser.borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([]));
    });

    /**
     * Tests the worst case fill scenario - candidate with large sums of collateral are entirely replaced
     * by subsequent better fills. Candidates are submitted by fillers in best -> worst order
     *
     * where x = number of fill accounts which can be submitted by a filler
     * where y = the redemption order candidate user queue size
     *
     * We create y * 2 users with descending (best -> worst) collateral ratios, (y * 2) / x fillers (rounded up) and 1 redemption order.
     *
     * Each filler fills with better candidates than the previous:
     *  - filler 1 - users             0 -> (x - 1)
     *  - filler 2 - users             x -> (x * 2) - 1
     *  - filler 3 - users       (x * 2) -> (x * 3) - 1
     *  - filler 4 - users       (x * 3) -> (x * 4) - 1
     *  - ...
     *  - filler n - users   x * (n - 1) -> (x * n) - 1
     *
     * Results in fill order:
     *
     *  - filler n       - users       (x * n) - 1 -> (x * n) - x
     *  - filler n - 1   - users (x * (n - 1)) - 1 -> (x * (n - 1)) - x)
     *  - filler n - 2   - users (x * (n - 2)) - 1 -> (x * (n - 2)) - x)
     */
    it('tests_redemption_fill_stress_test', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const numberOfUsers = REDEMPTION_CANDIDATE_QUEUE_SIZE * 2;
        const numberOfFillers = Math.ceil(numberOfUsers / FILL_INST_METADATA_ACCS_SIZE);

        const loanees = await newLoaneesDescendingCollateralRatio(env, borrowingGlobalAccounts, stakingPoolAccounts, numberOfUsers, pythPrices);

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, 2010, 10)

        const fillers = await operations_redemption.newFillUsers(env, borrowingGlobalAccounts, numberOfFillers)

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, 2005);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, 2005);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 5);

        for (let i = 0; i < numberOfFillers; i++) {
            const startUser = i * FILL_INST_METADATA_ACCS_SIZE;
            const endUser = Math.min(numberOfUsers, ((i + 1) * FILL_INST_METADATA_ACCS_SIZE));
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillers[i], 0,
                takeRangeOfLoanees(loanees, startUser, endUser)
            )
            if (endUser === numberOfUsers) {
                break;
            }
        }

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);

        let borrowerAndFillerMetadatas: Array<{ fillerMetadata: PublicKey, loaneeMetadata: PublicKey }> = [];
        for (let i = numberOfFillers; i > 0; i--) {
            const firstUser = numberOfUsers - REDEMPTION_CANDIDATE_QUEUE_SIZE;
            const startUser = Math.max(FILL_INST_METADATA_ACCS_SIZE * (i - 1), firstUser);
            const endUser = Math.min(FILL_INST_METADATA_ACCS_SIZE * i, numberOfUsers);
            borrowerAndFillerMetadatas = [
                ...borrowerAndFillerMetadatas,
                ...createExpectedFillRange(loanees, fillers[i - 1], startUser, endUser).reverse(),
            ];
            if (startUser === firstUser) {
                break;
            }
        }

        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts, borrowerAndFillerMetadatas);
    });

    /**
     * Tests the worst case clear scenario - redemption order is filled by multiple candidates with equal collateral.
     * Each candidate is submitted by a different filler.
     *
     * Where n is $REDEMPTION_QUEUE_LENGTH
     *
     * We create n + 1 users with equal collateral ratios, n fillers and 1 redemption order.
     *
     * Each filler fills with an equal candidate to the previous:
     *  - filler1 - user 0
     *  - filler2 - user 1
     *  - filler3 - user 2
     *  - ...
     *  - fillern - user n - 1
     *
     * The candidate users array has length n, so the fill result should be:
     *  - filler1 - user 0
     *  - filler2 - user 1
     *  - filler3 - user 2
     *  - ...
     *  - fillern - user n - 1
     *
     *  8 claims will be needed to clear the order completely:
     *  - claim 1 - fillers1-5 users 0-4
     *  - claim 2 - fillers6-10 users 5-9
     */

    it('tests_redemption_clear_stress_test', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const loaneeBorrowAmount = 3000;
        const loaneeFee = loaneeBorrowAmount * 0.005;
        const loaneeDebt = loaneeBorrowAmount + loaneeFee;
        const numberOfLoanees = REDEMPTION_CANDIDATE_QUEUE_SIZE + 1; // extra borrower to not redeem more than borrowed
        const redeemAmount = REDEMPTION_CANDIDATE_QUEUE_SIZE * loaneeDebt;

        const loanees = await newLoaneesEqualCollateralRatio(env, borrowingGlobalAccounts, stakingPoolAccounts, numberOfLoanees, loaneeBorrowAmount, pythPrices);

        const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingGlobalAccounts, redeemAmount, 10)

        const fillUsers = await operations_redemption.newFillUsers(env, borrowingGlobalAccounts, REDEMPTION_CANDIDATE_QUEUE_SIZE)

        const clearUser = await operations_redemption.newClearUser(env, borrowingGlobalAccounts)

        await operations_redemption.add_redemption_order(provider, program, borrowingGlobalAccounts, redemptionUser, pythPrices, redeemAmount);

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);
        await assertBurningVaultBalance(provider, borrowingGlobalAccounts.burningVault, redeemAmount);
        await assertBorrowerBalance(provider, program, redemptionUser.borrower, redemptionUser.borrowerAccounts, borrowingGlobalAccounts, 0, redemptionUser.borrowerInitialBalance, 0);

        for (let i = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i++) {
            await operations_redemption.fill_redemption_order(provider, program, borrowingGlobalAccounts, fillUsers[i], 0,
                takeRangeOfLoanees(loanees, i, i + 1))
        }

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 1);

        let expected = [];
        for (let i = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i++) {
            expected.push(...createExpectedFillRange(loanees, fillUsers[i], i, i + 1))
        }

        await assertRedemptionsQueueOrderFilled(provider, program, borrowingGlobalAccounts, expected);

        // wait for order to be clearable
        await waitToClear();

        for (let i = 0, batch = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i += batch) {
            // divide by 2 for 1:1 filler:borrower ratio
            batch = Math.min(Math.floor(CLEAR_INST_METADATA_ACCS_SIZE / 2), REDEMPTION_CANDIDATE_QUEUE_SIZE - i);
            console.log(`Clearing batch ${i} -> ${i + batch}`)
            const borrowerAndFillerMetadatas = [
                ...takeRangeOfLoanees(loanees, i, i + batch),
                ...takeRangeOfLoanees(fillUsers, i, i + batch),
            ];
            await clearWithRetry(provider, program, borrowingGlobalAccounts, clearUser, redemptionUser, 0, borrowerAndFillerMetadatas);
        }

        await assertRedemptionsQueueSize(provider, program, borrowingGlobalAccounts, 0);
        // Only 1 user's debt remains
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, loaneeDebt);

        // assert redeemed borrower balances
        for (let i = 0; i < REDEMPTION_CANDIDATE_QUEUE_SIZE; i++) {
            await assertBorrowerBalance(provider, program, loanees[i].borrower, loanees[i].borrowerAccounts, borrowingGlobalAccounts,
                0, loanees[i].borrowerInitialBalance - 0.5, loaneeBorrowAmount);
        }
        // extra borrower still has starting debt and balance
        await assertBorrowerBalance(provider, program, loanees[REDEMPTION_CANDIDATE_QUEUE_SIZE].borrower, loanees[REDEMPTION_CANDIDATE_QUEUE_SIZE].borrowerAccounts, borrowingGlobalAccounts,
            loaneeDebt, loanees[REDEMPTION_CANDIDATE_QUEUE_SIZE].borrowerInitialBalance - 0.5, loaneeBorrowAmount);
    });
})

/**
 Best -> worst collateral ratio
 */
const newLoaneesDescendingCollateralRatio = async (
    env: set_up.Env,
    globalAccounts: BorrowingGlobalAccounts,
    stakingPoolAccounts: StakingPoolAccounts,
    numberOfLoanees: number,
    pythPrices: set_up.PythPrices
): Promise<Array<BorrowingUserState>> => {
    const promises = new Array<Promise<BorrowingUserState>>();
    for (let i = 0; i < numberOfLoanees; i++) {
        promises.push(newLoanee(env, globalAccounts, stakingPoolAccounts, pythPrices, (i + 1) * 100_000, new Map<CollateralToken, number>([
            ["SOL", 0.5],
            ["BTC", 9_000_000],
            ["SRM", 9_000_000],
            ["ETH", 9_000_000],
            ["FTT", 9_000_000],
            ["RAY", 9_000_000],
        ])));
    }
    const loanees = await Promise.all(promises);
    return loanees.sort((a, b) => a.borrowerInitialDebt - b.borrowerInitialDebt);
}

const newLoaneesEqualCollateralRatio = async (
    env: set_up.Env,
    globalAccounts: BorrowingGlobalAccounts,
    stakingPoolAccounts: StakingPoolAccounts,
    numberOfUsers: number,
    borrowAmount: number,
    pythPrices: set_up.PythPrices
): Promise<Array<BorrowingUserState>> => {
    const promises = new Array<Promise<BorrowingUserState>>();
    for (let i = 0; i < numberOfUsers; i++) {
        promises.push(newLoanee(env, globalAccounts, stakingPoolAccounts, pythPrices, borrowAmount, new Map<CollateralToken, number>([
            ["SOL", 0.5],
            ["BTC", 1],
            ["SRM", 1],
            ["ETH", 1],
            ["FTT", 1],
            ["RAY", 1],
        ])));
    }
    const loanees = await Promise.all(promises);
    return loanees.sort((a, b) => a.borrowerInitialDebt - b.borrowerInitialDebt);
}

const takeRangeOfLoanees = (loanees: Array<BorrowingUserState>, start: number, end: number): Array<PublicKey> => {
    const loaneeRange = new Array<PublicKey>()
    for (let i = start; i < end; i++) {
        loaneeRange.push(loanees[i].borrowerAccounts.userMetadata.publicKey);
    }
    return loaneeRange;
}

const createExpectedFillRange = (loanees: Array<BorrowingUserState>, filler: BorrowingUserState, start: number, end: number
): Array<{ fillerMetadata: PublicKey, loaneeMetadata: PublicKey }> => {
    const loaneeRange = new Array<{ fillerMetadata: PublicKey, loaneeMetadata: PublicKey }>()
    for (let i = start; i < end; i++) {
        loaneeRange.push({
            fillerMetadata: filler.borrowerAccounts.userMetadata.publicKey,
            loaneeMetadata: loanees[i].borrowerAccounts.userMetadata.publicKey,
        });
    }
    return loaneeRange;
}

const waitToClear = async (): Promise<void> => {
    await sleep(REDEMPTION_CLEAR_WAIT_TIME);
}

export const waitAndClear = async (provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    clearer: BorrowingUserState,
    redeemer: BorrowingUserState,
    orderId: number,
    borrowerAndFillerMetadatas: PublicKey[],
): Promise<TransactionSignature> => {
    await waitToClear();
    return clearWithRetry(provider, program, borrowingGlobalAccounts, clearer, redeemer, orderId, borrowerAndFillerMetadatas);
}

const clearWithRetry = async (provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    clearer: BorrowingUserState,
    redeemer: BorrowingUserState,
    orderId: number,
    borrowerAndFillerMetadatas: PublicKey[],
): Promise<TransactionSignature> => {
    let endTime = Date.now() + REDEMPTION_CLEAR_WAIT_TIME;
    let lastErr = null;

    while (true) {
        try {
            return await operations_redemption.clear_redemption_order(provider, program, borrowingGlobalAccounts, clearer, redeemer, orderId, borrowerAndFillerMetadatas);
        } catch (e) {
            lastErr = e;
        }
        if (Date.now() > endTime) {
            console.log("Unable to clear redemption order!");
            throw lastErr;
        }
        console.log("Unable to clear redemption order, retrying...")
        await sleep(100)
    }
}


export async function getRedemptionsQueueSize(
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
): Promise<number> {
    const redemptionOrders = await getRedemptionsQueueData(program, borrowingGlobalAccounts.redemptionsQueue);
    const activeOrders = redemptionOrders.filter(order => order.status !== 0);
    return activeOrders.length;
}