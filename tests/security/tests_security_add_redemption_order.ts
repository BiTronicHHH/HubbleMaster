import * as instructions_redeem from '../../src/instructions_redeem';
import { getAddRedemptionOrderAccounts } from '../../src/instructions_redeem';
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import * as set_up from "../../src/set_up";
import { BorrowingGlobalAccounts, BorrowingUserState, PythPrices, setUpProgram } from "../../src/set_up";
import { PublicKey, Transaction } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Program, Provider } from "@project-serum/anchor";
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import { CollateralToken } from "../types";
import * as operations_redemption from "../operations_redemption";
import { newRedemptionUser } from "../operations_redemption";
import * as utils from "../../src/utils";
import { decimalToU64 } from "../../src/utils";

chai.use(chaiAsPromised)

describe('tests_security_add_redemption_order', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    let pythPrices: PythPrices;
    beforeEach('set_up_prices', async () => {
        pythPrices = await set_up.setUpPrices(
            provider,
            pyth,
            {
                solPrice: 10.0,
                ethPrice: 10.0,
                btcPrice: 10.0,
                srmPrice: 10.0,
                rayPrice: 10.0,
                fttPrice: 10.0,
            }
        );
    })

    it('security_add_redemption_order_without_redeemer_signing', async () => {
        const {
            borrowingAccounts,
            redeemAmount,
            redemptionUser: { borrower: redeemer1, borrowerAccounts: redeemer1Accounts },
        } = await newRedemptionScenario(env, pythPrices);

        const { borrower: redeemer2 } = await newRedemptionUser(env, borrowingAccounts, redeemAmount, 5);

        // redeemer2 sends an account which it does not sign for
        const ix = await program.instruction.addRedemptionOrder(
            new anchor.BN(decimalToU64(redeemAmount)), {
            accounts: getAddRedemptionOrderAccounts(
                redeemer1,
                redeemer1Accounts.userMetadata.publicKey,
                redeemer1Accounts.stablecoinAta,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                borrowingAccounts.redemptionsQueue,
                borrowingAccounts.burningVault,
                pythPrices,
            ),
            remainingAccounts: [
                {
                    pubkey: redeemer2.publicKey, // add redeemer2 as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [redeemer2], // redeemer2 signs
        });

        const tx = new Transaction();
        tx.add(ix);

        for (let i = 0; i < ix.keys.length; i++) {
            // mark redeemer1 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === redeemer1.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        await expect(utils.send(provider, tx, redeemer2.publicKey, [redeemer2]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });

    it('security_add_redemption_order_with_incorrect_user_metadata', async () => {
        const {
            borrowingAccounts,
            redeemAmount,
            redemptionUser: { borrower: redeemer1, borrowerAccounts: redeemer1Accounts },
        } = await newRedemptionScenario(env, pythPrices);

        const { borrowerAccounts: redeemer2Accounts } = await newRedemptionUser(env, borrowingAccounts, redeemAmount, 5);

        // redeemer1 specifies redeemer2 userMetadata
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer1,
            redeemer2Accounts.userMetadata.publicKey, // redeemer2 userMetadata
            redeemer1Accounts.stablecoinAta,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.redemptionsQueue,
            borrowingAccounts.burningVault,
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A raw constraint was violated");
    });

    it('security_add_redemption_order_with_incorrect_stablecoin_ata', async () => {
        const {
            borrowingAccounts,
            redeemAmount,
            redemptionUser: { borrower: redeemer1, borrowerAccounts: redeemer1Accounts },
        } = await newRedemptionScenario(env, pythPrices);

        const { borrowerAccounts: redeemer2Accounts } = await newRedemptionUser(env, borrowingAccounts, redeemAmount, 5);

        // redeemer1 specifies redeemer2 stablecoinAta
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer1,
            redeemer1Accounts.userMetadata.publicKey,
            redeemer2Accounts.stablecoinAta, // redeemer2 stablecoinAta
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.redemptionsQueue,
            borrowingAccounts.burningVault,
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A raw constraint was violated");
    });

    it('security_add_redemption_order_with_incorrect_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            redeemAmount,
            redemptionUser: { borrower: redeemer, borrowerAccounts: redeemerAccounts },
        } = await newRedemptionScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newRedemptionScenario(env, pythPrices);

        // redeemer specifies borrowingAccounts2 borrowingMarketState
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer,
            redeemerAccounts.userMetadata.publicKey,
            redeemerAccounts.stablecoinAta,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowingMarketState
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts1.burningVault,
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_add_redemption_order_with_incorrect_borrowing_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            redeemAmount,
            redemptionUser: { borrower: redeemer, borrowerAccounts: redeemerAccounts },
        } = await newRedemptionScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newRedemptionScenario(env, pythPrices);

        // redeemer specifies borrowingAccounts2 borrowingVaults
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer,
            redeemerAccounts.userMetadata.publicKey,
            redeemerAccounts.stablecoinAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey, // borrowingAccounts2 borrowingVaults
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts1.burningVault,
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_add_redemption_order_with_incorrect_redemptions_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            redeemAmount,
            redemptionUser: { borrower: redeemer, borrowerAccounts: redeemerAccounts },
        } = await newRedemptionScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newRedemptionScenario(env, pythPrices);

        // redeemer specifies borrowingAccounts2 redemptionsQueue
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer,
            redeemerAccounts.userMetadata.publicKey,
            redeemerAccounts.stablecoinAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts2.redemptionsQueue, // borrowingAccounts2 redemptionsQueue
            borrowingAccounts1.burningVault,
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_add_redemption_order_with_incorrect_burning_vault', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            redeemAmount,
            redemptionUser: { borrower: redeemer, borrowerAccounts: redeemerAccounts },
        } = await newRedemptionScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newRedemptionScenario(env, pythPrices);

        // redeemer specifies borrowingAccounts2 burningVault
        await expect(instructions_redeem.addRedemptionOrder(
            program,
            redeemer,
            redeemerAccounts.userMetadata.publicKey,
            redeemerAccounts.stablecoinAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts2.burningVault, // borrowingAccounts2 burningVault
            pythPrices,
            decimalToU64(redeemAmount),
        )).to.be.rejectedWith("A has_one constraint was violated");
    });
});

const newRedemptionScenario = async (
    env: set_up.Env,
    pythPrices: PythPrices,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    redeemAmount: number,
    redemptionUser: BorrowingUserState,
}> => {
    // Set up global accounts
    const { borrowingAccounts, stakingPoolAccounts } = await operations_borrowing.initialiseBorrowingMarkets(env);

    // To basically have a low impact on the redemption fee
    // due to redemption amount being much lower than total supply
    const whaleBorrow = 10000000.0;
    await operations_borrowing.newLoanee(
        env,
        borrowingAccounts,
        stakingPoolAccounts,
        pythPrices,
        whaleBorrow,
        new Map<CollateralToken, number>([["ETH", 10000000],])
    );

    const redeemAmount = 2000;

    await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices,
        redeemAmount, new Map<CollateralToken, number>([
            ["SOL", 50],
            ["ETH", 50],
            ["RAY", 50],
            ["SRM", 50],
            ["FTT", 50],
        ]));

    const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingAccounts, redeemAmount, 10)

    return {
        borrowingAccounts,
        redeemAmount,
        redemptionUser,
    }
}
