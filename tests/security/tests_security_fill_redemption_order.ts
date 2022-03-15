import * as instructions_redeem from '../../src/instructions_redeem';
import { getAddRedemptionOrderAccounts, getFillRedemptionOrderAccounts, getMetadataAccounts } from '../../src/instructions_redeem';
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
import { newFillUser, newRedemptionUser } from "../operations_redemption";
import * as utils from "../../src/utils";
import { decimalToU64 } from "../../src/utils";
import { FILL_INST_METADATA_ACCS_SIZE } from "../tests_redemption";
import BN from "bn.js";

chai.use(chaiAsPromised)

describe('tests_security_fill_redemption_order', () => {
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

    it('security_fill_redemption_order_without_filler_signing', async () => {
        const {
            borrowingAccounts,
            orderId,
            fillUser: { borrower: filler1, borrowerAccounts: filler1Accounts },
            borrowingUserMetadatas,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        const { borrower: filler2 } = await newFillUser(env, borrowingAccounts);
        // filler2 sends an account which it does not sign for
        const ix = await program.instruction.fillRedemptionOrder(
            new BN(orderId), {
            accounts: getFillRedemptionOrderAccounts(
                filler1,
                filler1Accounts.userMetadata.publicKey,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.redemptionsQueue,
            ),
            remainingAccounts: [
                {
                    pubkey: filler2.publicKey, // add filler2 as signer
                    isWritable: true,
                    isSigner: true
                },
                ...getMetadataAccounts(borrowingUserMetadatas, FILL_INST_METADATA_ACCS_SIZE),
            ],
            signers: [filler2], // filler2 signs
        });

        const tx = new Transaction();
        tx.add(ix);

        for (let i = 0; i < ix.keys.length; i++) {
            // mark filler1 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === filler1.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        await expect(utils.send(provider, tx, filler2.publicKey, [filler2]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });

    it('security_fill_redemption_order_with_incorrect_user_metadata', async () => {
        const {
            borrowingAccounts,
            orderId,
            fillUser: { borrower: filler1, borrowerAccounts: filler1Accounts },
            borrowingUserMetadatas,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        const { borrowerAccounts: filler2Accounts } = await newFillUser(env, borrowingAccounts);

        // filler1 specifies filler2 userMetadata
        await expect(instructions_redeem.fillRedemptionOrder(
            program,
            filler1,
            filler2Accounts.userMetadata.publicKey, // filler2 userMetadata
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.redemptionsQueue,
            orderId,
            borrowingUserMetadatas,
        )).to.be.rejectedWith("A raw constraint was violated");
    });

    it('security_fill_redemption_order_with_incorrect_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            fillUser: { borrower: filler, borrowerAccounts: fillerAccounts },
            borrowingUserMetadatas,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        // filler specifies borrowingAccounts2 borrowingMarketState
        await expect(instructions_redeem.fillRedemptionOrder(
            program,
            filler,
            fillerAccounts.userMetadata.publicKey,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowingMarketState
            borrowingAccounts1.redemptionsQueue,
            orderId,
            borrowingUserMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_fill_redemption_order_with_incorrect_redemptions_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            fillUser: { borrower: filler, borrowerAccounts: fillerAccounts },
            borrowingUserMetadatas,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        // filler specifies borrowingAccounts2 redemptionsQueue
        await expect(instructions_redeem.fillRedemptionOrder(
            program,
            filler,
            fillerAccounts.userMetadata.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.redemptionsQueue, // borrowingAccounts2 redemptionsQueue
            orderId,
            borrowingUserMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_fill_redemption_order_with_incorrect_candidate_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            fillUser: { borrower: filler, borrowerAccounts: fillerAccounts },
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        const {
            borrowingUserMetadatas: borrowingAccounts2Candidates,
        } = await newFillRedemptionOrderScenario(env, initialMarketOwner, pythPrices);

        // filler specifies borrowingAccounts2 candidateMetadatas
        await expect(instructions_redeem.fillRedemptionOrder(
            program,
            filler,
            fillerAccounts.userMetadata.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.redemptionsQueue,
            orderId,
            borrowingAccounts2Candidates, // borrowingAccounts2 candidateMetadatas
        )).to.be.rejectedWith("A has_one constraint was violated");
    });
});

const newFillRedemptionOrderScenario = async (
    env: set_up.Env,
    initialMarketOwner: PublicKey,
    pythPrices: PythPrices,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    orderId: number,
    fillUser: BorrowingUserState,
    borrowingUserMetadatas: PublicKey[],
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

    const borrowingUser = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices,
        redeemAmount, new Map<CollateralToken, number>([
            ["SOL", 50],
            ["ETH", 50],
            ["RAY", 50],
            ["SRM", 50],
            ["FTT", 50],
        ]));

    const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingAccounts, redeemAmount, 10)

    await operations_redemption.add_redemption_order(env.provider, env.program, borrowingAccounts, redemptionUser, pythPrices, redeemAmount);

    const fillUser = await newFillUser(env, borrowingAccounts);

    return {
        borrowingAccounts,
        orderId: 0,
        borrowingUserMetadatas: [borrowingUser.borrowerAccounts.userMetadata.publicKey],
        fillUser,
    }
}
