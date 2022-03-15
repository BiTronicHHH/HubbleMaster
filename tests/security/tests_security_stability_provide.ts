import * as operations_stability from "../operations_stability";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as instructions_stability from '../../src/instructions_stability';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { Env, setUpProgram } from "../../src/set_up";
import { initialiseStabilityPool, newStabilityPoolUser, } from "../operations_stability";
import { Keypair, Transaction } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { airdropStablecoin } from "../../src/instructions_borrow";
import { initialiseBorrowingMarkets, mintToAta } from "../operations_borrowing";
import { decimalToU64 } from "../../src/utils";

chai.use(chaiAsPromised)

describe('tests_security_stability_provide', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as Env;

    it('security_stability_provide_with_incorrect_stability_provider_state', async () => {
        const { borrowingAccounts, stabilityPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const stablecoinToProvide = 10;

        const { stabilityProvider: stabilityProvider1, stabilityProviderAccounts: stabilityProvider1Accounts } = await newStabilityPoolUser(provider, program, borrowingAccounts, stabilityPoolAccounts,
            stablecoinToProvide);

        const { stabilityProvider: stabilityProvider2, stabilityProviderAccounts: stabilityProvider2Accounts } = await newStabilityPoolUser(provider, program, borrowingAccounts, stabilityPoolAccounts,
            stablecoinToProvide);

        // stabilityProvider1 sends stabilityProvider2 stabilityProviderState
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider1.publicKey,
            stabilityProvider2Accounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            stabilityProvider1Accounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider1]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_borrowing_market_state', async () => {
        const { borrowingAccounts: borrowingAccounts1, stabilityPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        // setup second borrowing market
        const borrowingAccounts2 = (await initialiseBorrowingMarkets(env)).borrowingAccounts;

        const stablecoinToProvide = 10;

        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts1,
            stabilityPoolAccounts, stablecoinToProvide);

        // stabilityProvider sends incorrect borrowing market state
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts2.borrowingMarketState.publicKey, // borrowingAccounts2 borrowing market state
            borrowingAccounts1.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_stability_pool_state', async () => {
        const { borrowingAccounts, stabilityPoolAccounts: stabilityPool1Accounts } = await operations_stability.createMarketAndStabilityPool(env);

        // take a copy with a new stabilityPoolStateKey
        const borrowingAccountsCopy = { ...borrowingAccounts, stabilityPoolState: new Keypair() };

        const stabilityPool2Accounts = await initialiseStabilityPool(
            provider,
            program,
            initialMarketOwner,
            borrowingAccountsCopy,
        );

        const stablecoinToProvide = 10;

        // stabilityProvider registers with stabilityPool1
        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPool1Accounts, stablecoinToProvide);

        // stabilityProvider sends stabilityPool2 state
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccountsCopy.stabilityPoolState.publicKey, // stabilityPool2 state
            stabilityPool2Accounts.stabilityVaults.publicKey,
            stabilityPool2Accounts.epochToScaleToSum,
            stabilityPool2Accounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_epoch_to_scale_sum', async () => {
        const { borrowingAccounts, stabilityPoolAccounts: stabilityPool1Accounts } = await operations_stability.createMarketAndStabilityPool(env);

        // take a copy with a new stabilityPoolStateKey
        const borrowingAccountsCopy = { ...borrowingAccounts, stabilityPoolState: new Keypair() };

        const stabilityPool2Accounts = await initialiseStabilityPool(
            provider,
            program,
            initialMarketOwner,
            borrowingAccountsCopy,
        );

        const stablecoinToProvide = 10;

        // stabilityProvider registers with stabilityPool1
        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPool1Accounts, stablecoinToProvide);

        // stabilityProvider sends stabilityPool2 state
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccountsCopy.stabilityPoolState.publicKey, // stabilityPool2 state
            stabilityPool2Accounts.stabilityVaults.publicKey,
            stabilityPool2Accounts.epochToScaleToSum,
            stabilityPool2Accounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_stability_vaults', async () => {
        const { borrowingAccounts, stabilityPoolAccounts: stabilityPool1Accounts } = await operations_stability.createMarketAndStabilityPool(env);

        // take a copy with a new stabilityPoolStateKey
        const borrowingAccountsCopy = { ...borrowingAccounts, stabilityPoolState: new Keypair() };

        const stabilityPool2Accounts = await initialiseStabilityPool(
            provider,
            program,
            initialMarketOwner,
            borrowingAccountsCopy,
        );

        const stablecoinToProvide = 10;

        // stabilityProvider registers with stabilityPool1
        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPool1Accounts, stablecoinToProvide);

        // stabilityProvider sends stabilityPool2 vaults
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPool2Accounts.stabilityVaults.publicKey, // stabilityPool2 stabilityVaults
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_epoch_to_scale_sum', async () => {
        const { borrowingAccounts, stabilityPoolAccounts: stabilityPool1Accounts } = await operations_stability.createMarketAndStabilityPool(env);

        // take a copy with a new stabilityPoolStateKey
        const borrowingAccountsCopy = { ...borrowingAccounts, stabilityPoolState: new Keypair() };

        const stabilityPool2Accounts = await initialiseStabilityPool(
            provider,
            program,
            initialMarketOwner,
            borrowingAccountsCopy,
        );

        const stablecoinToProvide = 10;

        // stabilityProvider registers with stabilityPool1
        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPool1Accounts, stablecoinToProvide);

        // stabilityProvider sends stabilityPool2 epochToScaleToSum
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool2Accounts.epochToScaleToSum, // stabilityPool2 epochToScaleToSum
            stabilityPool1Accounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_stablecoin_stability_pool_vault', async () => {
        const { borrowingAccounts, stabilityPoolAccounts: stabilityPool1Accounts } = await operations_stability.createMarketAndStabilityPool(env);

        // take a copy with a new stabilityPoolStateKey
        const borrowingAccountsCopy = { ...borrowingAccounts, stabilityPoolState: new Keypair() };

        const stabilityPool2Accounts = await initialiseStabilityPool(
            provider,
            program,
            initialMarketOwner,
            borrowingAccountsCopy,
        );

        const stablecoinToProvide = 10;

        // stabilityProvider registers with stabilityPool1
        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPool1Accounts, stablecoinToProvide);

        // stabilityProvider sends stabilityPool2 stablecoinStabilityPoolVault
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPool1Accounts.stabilityVaults.publicKey,
            stabilityPool1Accounts.epochToScaleToSum,
            stabilityPool2Accounts.stablecoinStabilityPoolVault, // stabilityPool2 stablecoinStabilityPoolVault
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_stability_provide_with_incorrect_user_stablecoin_ata', async () => {
        const { borrowingAccounts, stabilityPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const stablecoinToProvide = 10;

        // stabilityProvider1 registers with stabilityPool1
        const { stabilityProvider: stabilityProvider1, stabilityProviderAccounts: stabilityProvider1Accounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPoolAccounts, stablecoinToProvide);

        // stabilityProvider2 registers with stabilityPool1
        const { stabilityProvider: stabilityProvider2, stabilityProviderAccounts: stabilityProvider2Accounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPoolAccounts, stablecoinToProvide);


        const ix = program.instruction.stabilityProvide(
            new anchor.BN(stablecoinToProvide), {
            accounts: instructions_stability.utils.getProvideStabilityAccounts(
                stabilityProvider1.publicKey,
                stabilityProvider1Accounts.stabilityProviderState.publicKey,
                borrowingAccounts.borrowingMarketState.publicKey,
                borrowingAccounts.stabilityPoolState.publicKey,
                stabilityPoolAccounts.stabilityVaults.publicKey,
                stabilityPoolAccounts.epochToScaleToSum,
                stabilityPoolAccounts.stablecoinStabilityPoolVault,
                stabilityProvider2Accounts.stablecoinAta, // stabilityProvider2's stablecoin ATA
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
            .to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_stability_provide_with_incorrect_user_mint_ata', async () => {
        const { borrowingAccounts, stabilityPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const stablecoinToProvide = 10;

        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPoolAccounts, stablecoinToProvide);

        await mintToAta(provider, borrowingAccounts, stabilityProviderAccounts, "ETH", decimalToU64(stablecoinToProvide));

        // stabilityProvider sends ETH ATA
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.ethAta, // ETH ATA
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_stability_provide_with_non_ata_stablecoin_account', async () => {
        const { borrowingAccounts, stabilityPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const stablecoinToProvide = 10;

        const { stabilityProvider, stabilityProviderAccounts } = await newStabilityPoolUser(provider, program, borrowingAccounts,
            stabilityPoolAccounts, stablecoinToProvide);

        const nonAtaStablecoinTokenAccount = await utils.createTokenAccount(provider, borrowingAccounts.stablecoinMint, stabilityProvider.publicKey);
        await airdropStablecoin(program, initialMarketOwner, borrowingAccounts.borrowingMarketState.publicKey, nonAtaStablecoinTokenAccount, borrowingAccounts.stablecoinMint, decimalToU64(stablecoinToProvide));

        // stabilityProvider sends non-ATA stablecoin account
        await expect(instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            nonAtaStablecoinTokenAccount, // Non-ATA stablecoin account
            utils.decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        )).to.be.rejectedWith("0x44d"); // ATA mismatch
    });
});
