import * as anchor from '@project-serum/anchor';
import * as operations_borrowing from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import { newBorrowingUser } from '../operations_borrowing';
import { CollateralToken, collateralTokenToNumber } from "../types";
import * as chai from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { expect } from "chai";
import { TokenInstructions } from "@project-serum/serum";
import { Transaction } from "@solana/web3.js";
import { Env, setUpProgram } from "../../src/set_up";

chai.use(chaiAsPromised)

describe('tests_security_deposit_collateral', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as Env;

    it('security_deposit_collateral_different_token_to_mint', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user specifies ETH but sends SRM mint details
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm, // SRM vault
                userAccounts.srmAta, // SRM ATA
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(10, "ETH"),
                [user],
                "ETH") // ETH token
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_deposit_collateral_native_instead_of_mint', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 1],
            ["SRM", 12],
        ]));

        // user specifies SRM but sends SOL mint details
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm, // SRM vault
                user.publicKey, // SOL account
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(0.5, "SRM"),
                [user],
                "SRM") // SRM token
        ).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_deposit_collateral_incorrect_metadata', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's metadata
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user1.publicKey,
                user2Accounts.userMetadata.publicKey, // user 2 metadata
                borrowingGlobalAccounts.collateralVaultEth,
                user1Accounts.ethAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(10, "ETH"),
                [user1],
                "ETH")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_deposit_collateral_incorrect_collateral_vault', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's ATA in place of collateral vault
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user1.publicKey,
                user1Accounts.userMetadata.publicKey,
                user2Accounts.ethAta, // user2 ATA instead of collateral vault
                user1Accounts.ethAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(10, "ETH"),
                [user1],
                "ETH")
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_deposit_collateral_incorrect_user_ata', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user2 passes their ATA and user1's account and metadata, user2 co-signs
        const ix = await program.instruction.depositCollateral(
            new anchor.BN(utils.collToLamports(10, "ETH")), new anchor.BN(collateralTokenToNumber("ETH")),
            {
                accounts: {
                    owner: user1.publicKey,
                    borrowingMarketState: borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingVaults: borrowingGlobalAccounts.borrowingVaults.publicKey,
                    userMetadata: user1Accounts.userMetadata.publicKey,
                    collateralFrom: user2Accounts.ethAta, // user2 ATA
                    collateralTo: borrowingGlobalAccounts.collateralVaultEth,
                    collateralTokenMint: borrowingGlobalAccounts.ethMint,
                    systemProgram: anchor.web3.SystemProgram.programId,
                    tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                },
                remainingAccounts: [
                    {
                        pubkey: user2.publicKey, // add user2 as signer
                        isWritable: true,
                        isSigner: true
                    }
                ],
                signers: [user1, user2] // user2 co-signs
            });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, user1.publicKey, [user1, user2]))
            .to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_deposit_collateral_incorrect_user_native_account', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 1],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 1],
        ]));

        // user1 passes user2's account, user2 co-signs
        const ix = await program.instruction.depositCollateral(
            new anchor.BN(utils.collToLamports(0.5, "SOL")), new anchor.BN(collateralTokenToNumber("SOL")),
            {
                accounts: {
                    owner: user1.publicKey,
                    borrowingMarketState: borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingVaults: borrowingGlobalAccounts.borrowingVaults.publicKey,
                    userMetadata: user1Accounts.userMetadata.publicKey,
                    collateralFrom: user2.publicKey, // user2 SOL account
                    collateralTo: borrowingGlobalAccounts.collateralVaultSol,
                    collateralTokenMint: borrowingGlobalAccounts.ethMint,
                    systemProgram: anchor.web3.SystemProgram.programId,
                    tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                },
                remainingAccounts: [
                    {
                        pubkey: user2.publicKey, // add user2 as signer
                        isWritable: true,
                        isSigner: true
                    }
                ],
                signers: [user1, user2] // user2 co-signs
            });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, user1.publicKey, [user1, user2]))
            .to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_deposit_collateral_different_borrowing_market', async () => {
        // 2 borrowing markets owned by same program
        const borrowingGlobalAccounts1 = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const borrowingGlobalAccounts2 = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;

        // user registers with borrowing market 1
        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newBorrowingUser(env,
            borrowingGlobalAccounts1, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));

        // user passes borrowingMarket2
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts2.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts2.borrowingMarketState.publicKey,
                borrowingGlobalAccounts2.borrowingVaults.publicKey,
                utils.collToLamports(0.5, "SOL"),
                [user],
                "SOL")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_deposit_collateral_different_borrowing_vaults', async () => {
        // 2 borrowing markets owned by same program
        const borrowingGlobalAccounts1 = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;
        const borrowingGlobalAccounts2 = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;

        // user registers with borrowing market 1
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newBorrowingUser(env,
            borrowingGlobalAccounts1, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));

        // user2 passes borrowingMarket2 borrowing vaults
        await expect(instructions_borrow
            .depositCollateral(
                program,
                user1.publicKey,
                user1Accounts.userMetadata.publicKey,
                borrowingGlobalAccounts2.collateralVaultSol,
                user1.publicKey,
                borrowingGlobalAccounts1.borrowingMarketState.publicKey,
                borrowingGlobalAccounts2.borrowingVaults.publicKey,
                utils.collToLamports(0.5, "SOL"),
                [user1],
                "SOL")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });
});
