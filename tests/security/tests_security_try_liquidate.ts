import * as operations_stability from "../operations_stability";
import { newLiquidator } from "../operations_stability";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import * as set_up from "../../src/set_up";
import { BorrowingGlobalAccounts, BorrowingUserAccounts, PythPrices, setUpProgram, StabilityPoolAccounts } from "../../src/set_up";
import { PublicKey, Transaction } from "@solana/web3.js";
import { Program, Provider } from "@project-serum/anchor";
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import { CollateralToken } from "../types";
import { PythUtils } from "../../src/pyth";
import { getBorrowingMarketState, getStabilityVaults } from "../data_provider";

chai.use(chaiAsPromised)

describe('tests_security_try_liquidate', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_try_liquidate_with_incorrect_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts: borrower1Accounts,
            liquidationPrices: liquidationPrices1,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
            borrowerAccounts: borrower2Accounts,
            liquidationPrices: liquidationPrices2,
        } = await newLiquidationScenario(env, pyth);
        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 borrowingMarketState
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowingMarketState
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrower1Accounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices1,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_stability_pool_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const { borrowingAccounts: borrowingAccounts2 } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 stabilityPoolState
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.stabilityPoolState.publicKey, // borrowingAccounts2 stabilityPoolState
            borrowerAccounts.userMetadata.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_user_metadata', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts: borrower1Accounts,
            liquidationPrices: liquidationPrices1,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
            borrowerAccounts: borrower2Accounts,
            liquidationPrices: liquidationPrices2,
        } = await newLiquidationScenario(env, pyth);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends a liquidatable borrowingAccounts2 userMetadata
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrower2Accounts.userMetadata.publicKey, // borrowingAccounts2 userMetadata
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices1,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_epoch_to_scale_sum', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends stabilityPool2 epochToScaleToSum
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool2Accounts.epochToScaleToSum, // stabilityPool2 epochToScaleToSum
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_stability_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends stabilityPool2 stabilityVaults
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool2Accounts.stabilityVaults.publicKey, // stabilityPool2 stabilityVaults
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_borrowing_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 borrowingVaults
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey, // borrowingAccounts2 borrowingVaults
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_liquidations_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 liquidationsQueue
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey,
            stabilityPool2Accounts.liquidationsQueue, // stabilityPool2 liquidationsQueue
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_stablecoin_stability_pool_vault', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 stablecoinStabilityPoolVault
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey,
            stabilityPool2Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault, // stabilityPool2 stablecoinStabilityPoolVault
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_stablecoin_stability_pool_vault', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // liquidator sends borrowingAccounts2 stablecoinStabilityPoolVault
        await expect(instructions_borrow.tryLiquidate(
            program,
            liquidator.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            borrowerAccounts.userMetadata.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            stabilityPool2Accounts.liquidationsQueue,
            borrowingAccounts1.stablecoinMint,
            stabilityPool1Accounts.stablecoinStabilityPoolVault, // stabilityPool2 stablecoinStabilityPoolVault
            liquidationPrices,
            [liquidator]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_try_liquidate_with_incorrect_stablecoin_mint', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const { liquidator, } = await newLiquidator(provider, program, borrowingAccounts1);

        // stabilityPool2 stablecoinMintAuthority
        const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingAccounts2.borrowingMarketState.publicKey);

        const { stablecoinStabilityPoolVaultAuthority } = await getStabilityVaults(program, stabilityPool2Accounts.stabilityVaults.publicKey);

        // liquidator sends stabilityPool2 stablecoinMint + stablecoinMintAuthority
        const ix = await program.instruction.tryLiquidate({
            accounts: instructions_borrow.utils.getTryLiquidateAccounts(
                liquidator.publicKey,
                borrowingAccounts1.borrowingMarketState.publicKey,
                borrowingAccounts1.stabilityPoolState.publicKey,
                borrowerAccounts.userMetadata.publicKey,
                stabilityPool1Accounts.epochToScaleToSum,
                stabilityPool2Accounts.stabilityVaults.publicKey,
                borrowingAccounts1.borrowingVaults.publicKey,
                stabilityPool1Accounts.liquidationsQueue,
                borrowingAccounts2.stablecoinMint, // stabilityPool2 stablecoinMint
                stablecoinMintAuthority, // stabilityPool2 stablecoinMintAuthority
                stabilityPool2Accounts.stablecoinStabilityPoolVault, // stabilityPool2 stablecoinStabilityPoolVault
                stablecoinStabilityPoolVaultAuthority, // stabilityPool2 stablecoinStabilityPoolVault
                liquidationPrices,
            ),
            signers: [liquidator],
        });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, liquidator.publicKey, [liquidator]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_try_liquidate_with_incorrect_liquidator', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            borrowerAccounts,
            liquidationPrices,
        } = await newLiquidationScenario(env, pyth);

        const { liquidator: liquidator1, } = await newLiquidator(provider, program, borrowingAccounts);
        const { liquidator: liquidator2, } = await newLiquidator(provider, program, borrowingAccounts);

        // stabilityPool2 stablecoinMintAuthority
        const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);

        const { stablecoinStabilityPoolVaultAuthority } = await getStabilityVaults(program, stabilityPoolAccounts.stabilityVaults.publicKey);

        // liquidator1 sends liquidator2
        const ix = await program.instruction.tryLiquidate({
            accounts: instructions_borrow.utils.getTryLiquidateAccounts(
                liquidator2.publicKey, // liquidator2
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.stabilityPoolState.publicKey,
                borrowerAccounts.userMetadata.publicKey,
                stabilityPoolAccounts.epochToScaleToSum,
                stabilityPoolAccounts.stabilityVaults.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                stabilityPoolAccounts.liquidationsQueue,
                borrowingAccounts.stablecoinMint,
                stablecoinMintAuthority,
                stabilityPoolAccounts.stablecoinStabilityPoolVault,
                stablecoinStabilityPoolVaultAuthority,
                liquidationPrices,
            ),
            remainingAccounts: [
                {
                    pubkey: liquidator2.publicKey, // add liquidator2 as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [liquidator1], // liquidator1 signs
        });

        for (let i = 0; i < ix.keys.length; i++) {
            // mark liquidator2 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === liquidator2.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, liquidator1.publicKey, [liquidator1]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });
});

const newLiquidationScenario = async (
    env: set_up.Env,
    pyth: PythUtils,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    stabilityPoolAccounts: StabilityPoolAccounts,
    borrowerAccounts: BorrowingUserAccounts,
    liquidationPrices: PythPrices,
}> => {
    const pythPrices = await set_up.setUpPrices(env.provider, pyth,
        {
            solPrice: 10.0,
            ethPrice: 10.0,
            btcPrice: 10.0,
            srmPrice: 10.0,
            rayPrice: 10.0,
            fttPrice: 10.0,
        }
    );

    // Set up global accouunts
    const { borrowingAccounts, stakingPoolAccounts } = await operations_borrowing.initialiseBorrowingMarkets(env);
    const stabilityPoolAccounts = await operations_stability.initialiseStabilityPool(env.provider, env.program, env.initialMarketOwner, borrowingAccounts);

    // we need one more, cannot liquidate the last user
    await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
        ["SOL", 1000]
    ]));

    const { borrowerAccounts } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices,
        2000, new Map<CollateralToken, number>([
            ["SOL", 50],
            ["ETH", 50],
            ["RAY", 50],
            ["SRM", 50],
            ["FTT", 50],
        ]));

    // Provide stability
    await operations_stability.newStabilityProvider(
        env.provider,
        env.program,
        borrowingAccounts,
        stabilityPoolAccounts,
        400,
    );


    // prices drop
    const liquidationPrices = await set_up.setUpPrices(env.provider, pyth,
        {
            solPrice: 7.6,
            ethPrice: 7.6,
            btcPrice: 7.6,
            srmPrice: 7.6,
            rayPrice: 7.6,
            fttPrice: 7.6,
        }
    );

    return {
        borrowingAccounts,
        stabilityPoolAccounts,
        borrowerAccounts,
        liquidationPrices,
    }
}
