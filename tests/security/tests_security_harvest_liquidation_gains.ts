import * as operations_stability from "../operations_stability";
import { getLiquidationRewardsVaultForToken, getStabilityProviderAtaForToken, newLiquidator, newStabilityProvider } from "../operations_stability";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as instructions_stability from '../../src/instructions_stability';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import * as set_up from "../../src/set_up";
import { BorrowingGlobalAccounts, setUpAssociatedTokenAccount, setUpProgram, StabilityPoolAccounts, StabilityProviderAccounts } from "../../src/set_up";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { Program, Provider } from "@project-serum/anchor";
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import { CollateralToken, stabilityTokenToNumber } from "../types";
import { PythUtils } from "../../src/pyth";
import { getBorrowingMarketState, getStabilityVaults } from "../data_provider";
import { solAirdropMin } from "../../src/utils";

chai.use(chaiAsPromised)

describe('tests_security_harvest_liquidation_gains', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_harvest_liquidation_gains_with_incorrect_vault_for_spl_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        // stabilityProvider specifies ETH but sends FTT details
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            stabilityPoolAccounts.liquidationRewardsVaultFtt, // FTT vault
            stabilityProviderAccounts.fttAta, // FTT ATA
            borrowingAccounts.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "ETH", // ETH token
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_harvest_liquidation_gains_with_native_vault_instead_of_spl_mint_vault', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        // stabilityProvider specifies SRM but sends SOL details
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            stabilityPoolAccounts.liquidationRewardsVaultSol, // SOL vault
            stabilityProvider.publicKey, // SOL account
            borrowingAccounts.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SRM", // SRM token
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_harvest_liquidation_gains_with_incorrect_spl_mint_vault_for_native_mint', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        // Deposit some SOL into the SRM vault
        await solAirdropMin(provider, stabilityPoolAccounts.liquidationRewardsVaultSrm, 10);

        // stabilityProvider specifies SRM but sends SOL details
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            stabilityPoolAccounts.liquidationRewardsVaultSrm, // SRM vault holding SOL
            stabilityProviderAccounts.srmAta, // SRM ATA can hold SOL
            borrowingAccounts.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL", // SOL token
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_harvest_liquidation_gains_with_incorrect_spl_mint_vault_and_authority', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: { stabilityProvider: stabilityProvider1, stabilityProviderAccounts: stabilityProvider1Accounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            stabilityProvider: stabilityProvider2,
            stabilityProviderAccounts: stabilityProvider2Accounts,
        } = await newStabilityProvider(provider, program, borrowingAccounts, stabilityPoolAccounts, 400);

        const { hbbMintAuthority } = await getBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);

        // stabilityProvider1 sends stabilityProvider2 ATA instead of vault
        const ix = await program.instruction.harvestLiquidationGains(
            stabilityTokenToNumber("ETH"), {
            accounts: instructions_stability.utils.getHarvestLiquidationGainsAccounts(
                stabilityProvider1.publicKey,
                stabilityProvider1Accounts.stabilityProviderState.publicKey,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                borrowingAccounts.stabilityPoolState.publicKey,
                stabilityPoolAccounts.stabilityVaults.publicKey,
                stabilityPoolAccounts.epochToScaleToSum,
                stabilityPoolAccounts.liquidationsQueue,
                stabilityProvider2Accounts.ethAta, // stabilityProvider2 ETH ATA instead of vault
                stabilityProvider2.publicKey, // stabilityProvider2 authority
                stabilityProvider1Accounts.ethAta,
                borrowingAccounts.hbbMint,
                hbbMintAuthority,
                stabilityProvider1Accounts.hbbAta,
            ),
            remainingAccounts: [
                {
                    pubkey: stabilityProvider2.publicKey, // add stabilityProvider2 as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [stabilityProvider1, stabilityProvider2], // stabilityProvider2 co-signs
        });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, stabilityProvider1.publicKey, [stabilityProvider1, stabilityProvider2]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_harvest_liquidation_gains_with_incorrect_borrowing_market_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newClearedLiquidationScenario(env, pyth);

        // stabilityProvider sends borrowingAccounts2 borrowingMarketState
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowingMarketState
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_borrowing_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
        } = await newClearedLiquidationScenario(env, pyth);

        // stabilityProvider sends borrowingAccounts2 borrowingVaults
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts2.borrowingVaults.publicKey, // borrowingAccounts2 borrowingVaults
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_stability_pool_state', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const { borrowingAccounts: borrowingAccounts2 } = await operations_stability.createMarketAndStabilityPool(env);

        // stabilityProvider sends stabilityPool2 stabilityPoolState
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts2.stabilityPoolState.publicKey, // stabilityPool2 stabilityPoolState
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_stability_vaults', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        // stabilityProvider sends stabilityPool2 stabilityVaults
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool2Accounts.stabilityVaults.publicKey, // stabilityPool2 stabilityVaults
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_epoch_to_scale_sum', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        // stabilityProvider sends stabilityPool2 epochToScaleToSum
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool2Accounts.epochToScaleToSum, // stabilityPool2 epochToScaleToSum
            stabilityPool1Accounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_liquidations_queue', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        // stabilityProvider sends stabilityPool2 liquidationsQueue
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts1.borrowingMarketState.publicKey,
            borrowingAccounts1.borrowingVaults.publicKey,
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool2Accounts.liquidationsQueue, // stabilityPool2 liquidationsQueue
            getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
            getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
            borrowingAccounts1.hbbMint,
            stabilityProviderAccounts.hbbAta,
            [stabilityProvider],
            "SOL",
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_harvest_liquidation_gains_with_incorrect_hbb_mint', async () => {
        const {
            borrowingAccounts: borrowingAccounts1,
            stabilityPoolAccounts: stabilityPool1Accounts,
            stabilityProvider: { stabilityProvider, stabilityProviderAccounts },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            borrowingAccounts: borrowingAccounts2,
            stabilityPoolAccounts: stabilityPool2Accounts,
        } = await operations_stability.createMarketAndStabilityPool(env);

        const stabilityPool2HbbAta = await setUpAssociatedTokenAccount(
            provider,
            stabilityProvider.publicKey,
            [stabilityProvider],
            stabilityProvider.publicKey,
            borrowingAccounts2.hbbMint
        );

        // stabilityPool2 hbbMintAuthority
        const { hbbMintAuthority } = await getBorrowingMarketState(program, borrowingAccounts2.borrowingMarketState.publicKey);

        const { liquidationRewardsVaultAuthority } = await getStabilityVaults(program, stabilityPool1Accounts.stabilityVaults.publicKey);

        // stabilityProvider sends borrowingAccounts2 hbbMint + hbbMintAuthority + ATA
        const ix = await program.instruction.harvestLiquidationGains(
            stabilityTokenToNumber("SOL"), {
            accounts: instructions_stability.utils.getHarvestLiquidationGainsAccounts(
                stabilityProvider.publicKey,
                stabilityProviderAccounts.stabilityProviderState.publicKey,
                borrowingAccounts1.borrowingMarketState.publicKey,
                borrowingAccounts1.borrowingVaults.publicKey,
                borrowingAccounts1.stabilityPoolState.publicKey,
                stabilityPool1Accounts.stabilityVaults.publicKey,
                stabilityPool1Accounts.epochToScaleToSum,
                stabilityPool1Accounts.liquidationsQueue,
                getLiquidationRewardsVaultForToken(stabilityPool1Accounts, "SOL"),
                liquidationRewardsVaultAuthority,
                getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, "SOL"),
                borrowingAccounts2.hbbMint, // stabilityPool2 hbbMint
                hbbMintAuthority, // stabilityPool2 hbbMintAuthority
                stabilityPool2HbbAta, // stabilityPool2 hbbAta
            ),
            signers: [stabilityProvider],
        });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, stabilityProvider.publicKey, [stabilityProvider]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_harvest_liquidation_gains_with_incorrect_hbb_ata', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: {
                stabilityProvider: stabilityProvider1,
                stabilityProviderAccounts: stabilityProvider1Accounts,
            },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            stabilityProvider: stabilityProvider2,
            stabilityProviderAccounts: stabilityProvider2Accounts,
        } = await newStabilityProvider(provider, program, borrowingAccounts, stabilityPoolAccounts, 400);

        // stabilityProvider1 sends stabilityProvider2 hbbAta
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider1.publicKey,
            stabilityProvider1Accounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPoolAccounts, "ETH"),
            getStabilityProviderAtaForToken(stabilityProvider1.publicKey, stabilityProvider1Accounts, "ETH"),
            borrowingAccounts.hbbMint,
            stabilityProvider2Accounts.hbbAta, // stabilityProvider2 hbbAta
            [stabilityProvider1],
            "SOL",
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_harvest_liquidation_gains_with_incorrect_stability_provider', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: {
                stabilityProvider: stabilityProvider1,
                stabilityProviderAccounts: stabilityProvider1Accounts,
            },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            stabilityProvider: stabilityProvider2,
            stabilityProviderAccounts: stabilityProvider2Accounts,
        } = await newStabilityProvider(provider, program, borrowingAccounts, stabilityPoolAccounts, 400);

        const { hbbMintAuthority } = await getBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);

        const { liquidationRewardsVaultAuthority } = await getStabilityVaults(program, stabilityPoolAccounts.stabilityVaults.publicKey);

        // stabilityProvider2 sends stabilityProvider1
        const ix = await program.instruction.harvestLiquidationGains(
            stabilityTokenToNumber("SOL"), {
            accounts: instructions_stability.utils.getHarvestLiquidationGainsAccounts(
                stabilityProvider1.publicKey,
                stabilityProvider1Accounts.stabilityProviderState.publicKey,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.borrowingVaults.publicKey,
                borrowingAccounts.stabilityPoolState.publicKey,
                stabilityPoolAccounts.stabilityVaults.publicKey,
                stabilityPoolAccounts.epochToScaleToSum,
                stabilityPoolAccounts.liquidationsQueue,
                getLiquidationRewardsVaultForToken(stabilityPoolAccounts, "SOL"),
                liquidationRewardsVaultAuthority,
                getStabilityProviderAtaForToken(stabilityProvider1.publicKey, stabilityProvider1Accounts, "SOL"),
                borrowingAccounts.hbbMint,
                hbbMintAuthority,
                stabilityProvider1Accounts.hbbAta,
            ),
            remainingAccounts: [
                {
                    pubkey: stabilityProvider2.publicKey, // add stabilityProvider2 as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [stabilityProvider2], // stabilityProvider2 signs
        });

        for (let i = 0; i < ix.keys.length; i++) {
            // mark stabilityProvider1 as a non-signer
            if (ix.keys[i].pubkey.toBase58() === stabilityProvider1.publicKey.toBase58()) {
                ix.keys[i].isSigner = false;
            }
        }

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, stabilityProvider2.publicKey, [stabilityProvider2]))
            .to.be.rejectedWith("0x8e"); // anchor ConstraintSigner
    });

    it('security_harvest_liquidation_gains_with_incorrect_stability_provider_ata', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: {
                stabilityProvider: stabilityProvider1,
                stabilityProviderAccounts: stabilityProvider1Accounts,
            },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            stabilityProvider: stabilityProvider2,
            stabilityProviderAccounts: stabilityProvider2Accounts,
        } = await newStabilityProvider(provider, program, borrowingAccounts, stabilityPoolAccounts, 400);

        // stabilityProvider1 sends stabilityProvider2's ATA
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider1.publicKey,
            stabilityProvider1Accounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPoolAccounts, "ETH"),
            stabilityProvider2Accounts.ethAta, // stabilityProvider2 ethAta
            borrowingAccounts.hbbMint,
            stabilityProvider1Accounts.hbbAta,
            [stabilityProvider1],
            "SOL",
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_harvest_liquidation_gains_with_incorrect_stability_provider_native_account', async () => {
        const {
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityProvider: {
                stabilityProvider: stabilityProvider1,
                stabilityProviderAccounts: stabilityProvider1Accounts,
            },
        } = await newClearedLiquidationScenario(env, pyth);

        const {
            stabilityProvider: stabilityProvider2,
            stabilityProviderAccounts: stabilityProvider2Accounts,
        } = await newStabilityProvider(provider, program, borrowingAccounts, stabilityPoolAccounts, 400);

        // stabilityProvider1 sends stabilityProvider2's account
        await expect(instructions_stability.harvestLiquidationGains(
            program,
            stabilityProvider1.publicKey,
            stabilityProvider1Accounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.liquidationsQueue,
            getLiquidationRewardsVaultForToken(stabilityPoolAccounts, "SOL"),
            stabilityProvider2.publicKey, // stabilityProvider2 account
            borrowingAccounts.hbbMint,
            stabilityProvider1Accounts.hbbAta,
            [stabilityProvider1],
            "SOL",
        )).to.be.rejectedWith("0x44c"); // key mismatch
    });
});

const newClearedLiquidationScenario = async (
    env: set_up.Env,
    pyth: PythUtils,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    stabilityPoolAccounts: StabilityPoolAccounts,
    stabilityProvider: { stabilityProvider: Keypair, stabilityProviderAccounts: StabilityProviderAccounts },
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
    const { stabilityProvider, stabilityProviderAccounts } = await operations_stability.newStabilityProvider(
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
        true);

    return {
        borrowingAccounts,
        stabilityPoolAccounts,
        stabilityProvider: {
            stabilityProvider,
            stabilityProviderAccounts,
        },
    }
}
