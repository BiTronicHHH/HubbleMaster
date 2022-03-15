import * as anchor from '@project-serum/anchor';
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import { solAirdropMin } from "../../src/utils";
import { CollateralToken, collateralTokenToNumber } from "../types";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { TokenInstructions } from "@project-serum/serum";
import { Transaction } from "@solana/web3.js";
import * as set_up from "../../src/set_up";
import { setUpProgram } from "../../src/set_up";

chai.use(chaiAsPromised)

describe('tests_security_withdraw_collateral', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    let pythPrices: any;
    beforeEach('set_up_prices', async () => {
        pythPrices = await set_up.setUpPrices(
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
    })

    it('security_withdraw_collateral_different_mint_vault_for_mint', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user specifies ETH but sends SRM details
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm, // SRM vault
                userAccounts.srmAta, // SRM ATA
                pythPrices,
                utils.collToLamports(10, "SRM"),
                [user],
                "ETH") // ETH token
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_withdraw_collateral_native_vault_instead_of_mint', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["SOL", 1],
            ["SRM", 12],
        ]));

        // user specifies ETH but sends SOL details
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol, // SOL vault
                user.publicKey, // SOL account
                pythPrices,
                utils.collToLamports(0.5, "SOL"),
                [user],
                "SRM") // SRM token
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_withdraw_collateral_different_mint_vault_for_native_mint', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // Deposit some SOL into the SRM vault
        await solAirdropMin(provider, borrowingGlobalAccounts.collateralVaultSrm, 1);

        // user specifies SOL but sends SRM details
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm, // SRM vault holding SOL
                userAccounts.srmAta, // SRM ATA can hold SOL
                pythPrices,
                utils.collToLamports(0.5, "SRM"),
                [user],
                "SOL") // SOL token
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_withdraw_collateral_incorrect_metadata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));

        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's metadata
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                user2Accounts.userMetadata.publicKey, // user 2 metadata
                borrowingGlobalAccounts.collateralVaultEth,
                user1Accounts.ethAta,
                pythPrices,
                utils.collToLamports(10, "ETH"),
                [user1],
                "ETH")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_withdraw_collateral_incorrect_metadata_and_ata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));

        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's metadata
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                user2Accounts.userMetadata.publicKey, // user 2 metadata
                borrowingGlobalAccounts.collateralVaultEth,
                user2Accounts.ethAta, // user 2 ATA
                pythPrices,
                utils.collToLamports(10, "ETH"),
                [user1],
                "ETH")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_withdraw_collateral_incorrect_collateral_vault_and_authority', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));

        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's ATA and authority instead of collateral vault, user2 co-signs
        const ix = await program.instruction.withdrawCollateral(
            new anchor.BN(utils.collToLamports(10, "ETH")), new anchor.BN(collateralTokenToNumber("ETH")),
            {
                accounts: {
                    owner: user1.publicKey,
                    borrowingMarketState: borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingVaults: borrowingGlobalAccounts.borrowingVaults.publicKey,
                    userMetadata: user1Accounts.userMetadata.publicKey,
                    collateralFrom: user2Accounts.ethAta, // user2 ATA instead of vault
                    collateralFromAuthority: user2.publicKey, // user2 authority
                    collateralTo: user1Accounts.ethAta,
                    systemProgram: anchor.web3.SystemProgram.programId,
                    tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                    pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
                    pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
                    pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
                    pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
                    pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
                    pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
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

    it('security_withdraw_collateral_incorrect_user_ata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
        ]));

        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 22],
        ]));

        // user1 passes user2's ATA
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                user1Accounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultEth,
                user2Accounts.ethAta, // user 2 ATA
                pythPrices,
                utils.collToLamports(10, "ETH"),
                [user1],
                "ETH")
        ).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_withdraw_collateral_incorrect_user_native_account', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["SOL", 1],
        ]));

        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["SOL", 1],
        ]));

        // user1 passes user2's account
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                user1Accounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user2.publicKey, // user2 account
                pythPrices,
                utils.collToLamports(0.5, "SOL"),
                [user1],
                "SOL")
        ).to.be.rejectedWith("0x44c"); // key mismatch
    });

    it('security_withdraw_collateral_different_borrowing_market', async () => {
        // 2 borrowing markets owned by same program
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        // user1 registers with borrowing market 1
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env,
            borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));
        // user2 registers with borrowing market 2 to add balance
        await newLoanee(env,
            borrowingGlobalAccounts2, stakingPoolAccounts2, pythPrices, 0, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));

        // user2 passes borrowingMarket2
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts2.borrowingMarketState.publicKey,
                borrowingGlobalAccounts2.borrowingVaults.publicKey,
                user1Accounts.userMetadata.publicKey,
                borrowingGlobalAccounts2.collateralVaultSol,
                user1.publicKey,
                pythPrices,
                utils.collToLamports(0.5, "SOL"),
                [user1],
                "SOL")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_withdraw_collateral_different_borrowing_vaults', async () => {
        // 2 borrowing markets owned by same program
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        // user1 registers with borrowing market 1
        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env,
            borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));
        // user2 registers with borrowing market 2 to add balance
        await newLoanee(env,
            borrowingGlobalAccounts2, stakingPoolAccounts2, pythPrices, 0, new Map<CollateralToken, number>([
                ["SOL", 1],
            ]));

        // user2 passes borrowingMarket2 borrowing vaults
        await expect(instructions_borrow
            .withdrawCollateral(
                program,
                user1.publicKey,
                borrowingGlobalAccounts1.borrowingMarketState.publicKey,
                borrowingGlobalAccounts2.borrowingVaults.publicKey,
                user1Accounts.userMetadata.publicKey,
                borrowingGlobalAccounts2.collateralVaultSol,
                user1.publicKey,
                pythPrices,
                utils.collToLamports(0.5, "SOL"),
                [user1],
                "SOL")
        ).to.be.rejectedWith("A has_one constraint was violated");
    });

});
