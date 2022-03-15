import * as instructions_redeem from '../../src/instructions_redeem';
import { getClearRedemptionOrderAccounts, getMetadataAccounts } from '../../src/instructions_redeem';
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import * as set_up from "../../src/set_up";
import { BorrowingGlobalAccounts, BorrowingUserState, PythPrices, setUpProgram } from "../../src/set_up";
import { PublicKey, Transaction } from "@solana/web3.js";
import { Program, Provider } from "@project-serum/anchor";
import * as operations_borrowing from "../operations_borrowing";
import { initialiseBorrowingMarkets, newLoanee } from "../operations_borrowing";
import { CollateralToken } from "../types";
import * as operations_redemption from "../operations_redemption";
import { newClearUser, newFillUser } from "../operations_redemption";
import * as utils from "../../src/utils";
import { FILL_INST_METADATA_ACCS_SIZE, waitAndClear } from "../tests_redemption";
import BN from "bn.js";
import { getBorrowingVaults } from "../data_provider";
import { airdropStablecoin } from "../../src/instructions_borrow";
import { decimalToU64 } from "../../src/utils";

chai.use(chaiAsPromised)

describe('tests_security_clear_redemption_order', () => {
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

    it('security_clear_redemption_order_without_clearer_signing', async () => {
        const {
            borrowingAccounts,
            orderId,
            clearer: { borrower: clearer1, borrowerAccounts: clearer1Accounts },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const { borrower: clearer2 } = await newClearUser(env, borrowingAccounts);

        const { burningVaultAuthority } = await getBorrowingVaults(program, borrowingAccounts.borrowingVaults.publicKey);

        // clearer2 sends an account which it does not sign for
        const ix = await program.instruction.clearRedemptionOrder(
            new BN(orderId), {
            accounts: getClearRedemptionOrderAccounts(
                clearer1,
                clearer1Accounts.userMetadata.publicKey,
                redeemerMetadata,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                borrowingAccounts.redemptionsQueue,
                borrowingAccounts.burningVault,
                burningVaultAuthority,
                borrowingAccounts.stablecoinMint,
            ),
            remainingAccounts: [
                {
                    pubkey: clearer2.publicKey, // add clearer2 as signer
                    isWritable: true,
                    isSigner: true
                },
                ...getMetadataAccounts(borrowerAndFillerMetadatas, FILL_INST_METADATA_ACCS_SIZE),
            ],
            signers: [clearer2], // clearer2 signs
        });

        const tx = new Transaction();
        tx.add(ix);

        for (let i = 0; i < ix.keys.length; i++) {
            // mark clearer1 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === clearer1.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        await expect(utils.send(provider, tx, clearer2.publicKey, [clearer2]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });

    it('security_clear_redemption_order_with_incorrect_clearer_user_metadata', async () => {
        const {
            borrowingAccounts,
            orderId,
            clearer: { borrower: clearer1 },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const { borrowerAccounts: clearer2Accounts } = await newClearUser(env, borrowingAccounts);

        // clearer1 specifies clearer2 userMetadata
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer1,
            clearer2Accounts.userMetadata.publicKey, // clearer2 userMetadata
            redeemerMetadata,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.redemptionsQueue,
            borrowingAccounts.burningVault,
            borrowingAccounts.stablecoinMint,
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("A raw constraint was violated");
    });

    it('security_clear_redemption_order_with_incorrect_redeemer_user_metadata', async () => {
        const {
            borrowingAccounts,
            orderId,
            clearer: { borrower: clearer1, borrowerAccounts: clearer1Accounts },
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const { borrowerAccounts: clearer2Accounts } = await newClearUser(env, borrowingAccounts);

        // clearer1 specifies clearer2 userMetadata instead of the order redeemer metadata
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer1,
            clearer1Accounts.userMetadata.publicKey,
            clearer2Accounts.userMetadata.publicKey, // clearer2 userMetadata
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.redemptionsQueue,
            borrowingAccounts.burningVault,
            borrowingAccounts.stablecoinMint,
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("Redeemer does not match with the order being redeemed");
    });

    it('security_clear_redemption_order_with_incorrect_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            clearer: { borrower: clearer, borrowerAccounts: clearerAccounts },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        // clearer specifies borrowingAccounts2 borrowingMarketState
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer,
            clearerAccounts.userMetadata.publicKey,
            redeemerMetadata,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowingMarketState
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts1.burningVault,
            borrowingAccounts1.stablecoinMint,
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_redemption_order_with_incorrect_borrowing_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            clearer: { borrower: clearer, borrowerAccounts: clearerAccounts },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        // clearer specifies borrowingAccounts2 borrowingVaults
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer,
            clearerAccounts.userMetadata.publicKey,
            redeemerMetadata,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey, // borrowingAccounts2 borrowingVaults
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts1.burningVault,
            borrowingAccounts1.stablecoinMint,
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_redemption_order_with_incorrect_redemptions_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            clearer: { borrower: clearer, borrowerAccounts: clearerAccounts },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        // clearer specifies borrowingAccounts2 redemptionsQueue
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer,
            clearerAccounts.userMetadata.publicKey,
            redeemerMetadata,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts2.redemptionsQueue, // borrowingAccounts2 redemptionsQueue
            borrowingAccounts1.burningVault,
            borrowingAccounts1.stablecoinMint,
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_redemption_order_with_incorrect_burning_vault_and_stablecoin_mint', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            clearer: { borrower: clearer, borrowerAccounts: clearerAccounts },
            redeemerMetadata,
            borrowerAndFillerMetadatas,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await initialiseBorrowingMarkets(env);
        // Ensure there are funds in the borrowingAccounts2 burningVault
        await airdropStablecoin(program, initialMarketOwner, borrowingAccounts2.borrowingMarketState.publicKey,
            borrowingAccounts2.burningVault, borrowingAccounts2.stablecoinMint, decimalToU64(50_000)
        )

        // clearer specifies borrowingAccounts2 burningVault + stablecoinMint
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer,
            clearerAccounts.userMetadata.publicKey,
            redeemerMetadata,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts2.burningVault, // borrowingAccounts2 burningVault
            borrowingAccounts2.stablecoinMint, // borrowingAccounts2 stablecoinMint
            orderId,
            borrowerAndFillerMetadatas,
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_redemption_order_with_incorrect_candidate_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            orderId,
            clearer: { borrower: clearer, borrowerAccounts: clearerAccounts },
            redeemerMetadata,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        const {
            borrowerAndFillerMetadatas: borrowingAccounts2Candidates,
        } = await newClearRedemptionOrderScenario(env, pythPrices);

        // clearer specifies borrowingAccounts2 candidateMetadatas
        await expect(instructions_redeem.clearRedemptionOrder(
            program,
            clearer,
            clearerAccounts.userMetadata.publicKey,
            redeemerMetadata,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.redemptionsQueue,
            borrowingAccounts1.burningVault,
            borrowingAccounts1.stablecoinMint,
            orderId,
            borrowingAccounts2Candidates, // borrowingAccounts2 candidateMetadatas
        )).to.be.rejectedWith("A has_one constraint was violated");
    });
});

const newClearRedemptionOrderScenario = async (
    env: set_up.Env,
    pythPrices: PythPrices,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    orderId: number,
    clearer: BorrowingUserState,
    redeemerMetadata: PublicKey,
    borrowerAndFillerMetadatas: PublicKey[],
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

    const fishRedeemAmount = 2000;
    const redeemAmount = 2000;

    const borrowingUser = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices,
        redeemAmount, new Map<CollateralToken, number>([
            ["SOL", 50],
            ["ETH", 50],
            ["RAY", 50],
            ["SRM", 50],
            ["FTT", 50],
        ]));

    // small holding user to redeem against first,
    // so that we can loop and ensure the redemption order is in fill mode
    const fishUser = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices,
        fishRedeemAmount, new Map<CollateralToken, number>([
            ["SOL", 115],
            ["ETH", 115],
        ]));

    const redemptionUser = await operations_redemption.newRedemptionUser(env, borrowingAccounts, redeemAmount + fishRedeemAmount, 10)

    await operations_redemption.add_redemption_order(env.provider, env.program, borrowingAccounts, redemptionUser, pythPrices, redeemAmount + fishRedeemAmount);

    const fillUser = await newFillUser(env, borrowingAccounts);

    const orderId = 0;

    await operations_redemption.fill_redemption_order(env.provider, env.program, borrowingAccounts, fillUser, orderId,
        [
            fishUser.borrowerAccounts.userMetadata.publicKey,
            borrowingUser.borrowerAccounts.userMetadata.publicKey
        ],
    );

    const clearer = await newClearUser(env, borrowingAccounts);

    // Clear the fish user to ensure order is in clearing state
    await waitAndClear(env.provider, env.program, borrowingAccounts, clearer, redemptionUser, orderId, [
        fishUser.borrowerAccounts.userMetadata.publicKey,
        fillUser.borrowerAccounts.userMetadata.publicKey,
    ])

    return {
        borrowingAccounts,
        orderId,
        clearer,
        redeemerMetadata: redemptionUser.borrowerAccounts.userMetadata.publicKey,
        borrowerAndFillerMetadatas: [
            borrowingUser.borrowerAccounts.userMetadata.publicKey,
            fillUser.borrowerAccounts.userMetadata.publicKey,
        ],
    }
}
