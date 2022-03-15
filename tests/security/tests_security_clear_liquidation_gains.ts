import * as operations_stability from "../operations_stability";
import { newLiquidator } from "../operations_stability";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as instructions_stability from '../../src/instructions_stability';
import * as utils from "../../src/utils";
import { createSolAccount, solAccountWithMinBalance, solAirdropMin } from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import * as set_up from "../../src/set_up";
import { BorrowingGlobalAccounts, LiquidatorAccounts, setUpProgram, StabilityPoolAccounts } from "../../src/set_up";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { Program, Provider } from "@project-serum/anchor";
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import { CollateralToken, stabilityTokenToNumber } from "../types";
import { PythUtils } from "../../src/pyth";
import { getBorrowingVaults } from "../data_provider";

chai.use(chaiAsPromised)

describe('tests_security_clear_liquidation_gains', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_clear_liquidation_gains_with_incorrect_collateral_vault_for_spl_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        // clearing agent specifies ETH but sends FTT details
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.fttAta, // FTT ATA
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts.collateralVaultFtt, // FTT collateralVault
            stabilityPoolAccounts.liquidationRewardsVaultFtt, // FTT liquidationRewardsVault
            [liquidator],
            "ETH", // ETH token
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_clear_liquidation_gains_with_incorrect_collateral_vault_for_native_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator },
        } = await newUnclearedLiquidationScenario(env, pyth);

        // Deposit some SOL into account owned by program
        const solAccount = await createSolAccount(provider, program.programId);
        await solAirdropMin(provider, solAccount, 10);

        // clearing agent sends different SOL account to collateral vault
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidator.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            solAccount, // Incorrect collateralVault
            stabilityPoolAccounts.liquidationRewardsVaultSol,
            [liquidator],
            "SOL", // SOL token
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_clear_liquidation_gains_with_incorrect_rewards_vault_for_spl_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const fttAta = await utils.createTokenAccount(provider, borrowingAccounts.fttMint,
            liquidator.publicKey);

        // clearing agent specifies FTT liquidations vault not owned by program
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.fttAta,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts.collateralVaultFtt,
            fttAta, // FTT account not owned by program for liquidationRewardsVault
            [liquidator],
            "FTT",
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_clear_liquidation_gains_with_incorrect_rewards_vault_for_native_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const { keyPair: solAccount } = await solAccountWithMinBalance(provider, 10);

        // clearing agent sends different SOL account to collateral vault
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidator.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts.collateralVaultSol,
            solAccount.publicKey, // SOL account not owned by program for liquidationRewardsVault
            [liquidator],
            "SOL",
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_clear_liquidation_gains_with_incorrect_claiming_agent_ata_for_spl_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator, },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const fttAta = await utils.createTokenAccount(provider, borrowingAccounts.fttMint, new Keypair().publicKey);

        // clearing agent specifies FTT ATA not associated with their account
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            fttAta, // FTT ATA not owned by clearingAgent
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts.collateralVaultFtt,
            stabilityPoolAccounts.liquidationRewardsVaultFtt,
            [liquidator],
            "FTT",
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_clear_liquidation_gains_with_incorrect_claiming_agent_ata_for_native_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const { keyPair: solAccount } = await solAccountWithMinBalance(provider, 10);

        // clearing agent sends different SOL account to their own
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            solAccount.publicKey, // SOL account not owned by clearingAgent
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts.collateralVaultSol,
            stabilityPoolAccounts.liquidationRewardsVaultSol,
            [liquidator],
            "SOL",
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_clear_liquidation_gains_without_liquidator_signing', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidator: { liquidator: liquidator1 },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const { liquidator: liquidator2, } = await newLiquidator(provider, program, borrowingAccounts);

        const { collateralVaultsAuthority } = await getBorrowingVaults(program, borrowingAccounts.borrowingVaults.publicKey);

        // clearing agent sends an account which it does not sign for
        const ix = await program.instruction.clearLiquidationGains(
            stabilityTokenToNumber("SOL"), {
            accounts: instructions_stability.utils.getClearLiquidationGainsAccounts(
                liquidator1.publicKey, // liquidator1
                liquidator1.publicKey,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                borrowingAccounts.stabilityPoolState.publicKey,
                stabilityPoolAccounts.stabilityVaults.publicKey,
                stabilityPoolAccounts.liquidationsQueue,
                borrowingAccounts.collateralVaultSol,
                collateralVaultsAuthority,
                stabilityPoolAccounts.liquidationRewardsVaultSol,
            ),
            remainingAccounts: [
                {
                    pubkey: liquidator2.publicKey, // add liquidator2 as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [liquidator2], // liquidator2 signs
        });

        const tx = new Transaction();
        tx.add(ix);

        for (let i = 0; i < ix.keys.length; i++) {
            // mark liquidator1 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === liquidator1.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        await expect(utils.send(provider, tx, liquidator2.publicKey, [liquidator2]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });

    it('security_clear_liquidation_gains_with_incorrect_borrowing_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newUnclearedLiquidationScenario(env, pyth);

        // clearing agent sends borrowingAccounts2 borrowingVaults
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.ethAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey, // borrowingAccounts2 borrowingVaults
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts1.collateralVaultEth,
            stabilityPoolAccounts.liquidationRewardsVaultEth,
            [liquidator],
            "ETH",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_liquidation_gains_with_incorrect_stability_pool_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newUnclearedLiquidationScenario(env, pyth);

        // clearing agent sends borrowingAccounts2 stabilityPoolState
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.ethAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts2.stabilityPoolState.publicKey, // borrowingAccounts2 stabilityPoolState
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.liquidationsQueue,
            borrowingAccounts1.collateralVaultEth,
            stabilityPoolAccounts.liquidationRewardsVaultEth,
            [liquidator],
            "ETH",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_liquidation_gains_with_incorrect_stability_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const {
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await newUnclearedLiquidationScenario(env, pyth);

        // clearing agent sends stabilityPool2 stabilityVaults
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.ethAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool2Accounts.stabilityVaults.publicKey, // stabilityPool2 stabilityVaults
            stabilityPool1Accounts.liquidationsQueue,
            borrowingAccounts1.collateralVaultEth,
            stabilityPool1Accounts.liquidationRewardsVaultEth,
            [liquidator],
            "ETH",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_clear_liquidation_gains_with_incorrect_liquidations_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            liquidator: { liquidator, liquidatorAccounts },
        } = await newUnclearedLiquidationScenario(env, pyth);

        const {
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await newUnclearedLiquidationScenario(env, pyth);

        // clearing agent sends stabilityPool2 stabilityVaults
        await expect(instructions_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            liquidatorAccounts.ethAta,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool2Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.liquidationsQueue, // stabilityPool2 liquidationsQueue
            borrowingAccounts1.collateralVaultEth,
            stabilityPool1Accounts.liquidationRewardsVaultEth,
            [liquidator],
            "ETH",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });
});

const newUnclearedLiquidationScenario = async (
    env: set_up.Env,
    pyth: PythUtils,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    stabilityPoolAccounts: StabilityPoolAccounts,
    liquidator: { liquidator: Keypair, liquidatorAccounts: LiquidatorAccounts },
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

    // Liquidate and clear
    const { liquidator, liquidatorAccounts } = await newLiquidator(env.provider, env.program, borrowingAccounts);
    await operations_stability.tryLiquidate(env.program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, liquidationPrices,
        false);

    return {
        borrowingAccounts,
        stabilityPoolAccounts,
        liquidator: {
            liquidator,
            liquidatorAccounts
        },
    }
}
