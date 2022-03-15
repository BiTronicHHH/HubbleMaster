import * as anchor from '@project-serum/anchor';
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import { airdropStablecoin } from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import { CollateralToken } from "../types";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { Transaction } from "@solana/web3.js";
import * as set_up from "../../src/set_up";
import { setUpAssociatedStablecoinAccount, setUpProgram } from "../../src/set_up";
import { getBorrowingMarketState, getBorrowingVaults } from "../data_provider";

chai.use(chaiAsPromised)

describe('tests_security_repay_loan', () => {
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
                rayPrice: 10000.0,
                fttPrice: 10000.0,
            }
        );
    })

    it('security_repay_loan_incorrect_borrowing_market_state', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // deposit into borrowingMarketState2
        await newLoanee(env, borrowingGlobalAccounts2, stakingPoolAccounts2, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user sends borrowingMarketState2
        await expect(instructions_borrow.repayLoan(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts1.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts2.borrowingMarketState.publicKey, // borrowingMarketState2
            borrowingGlobalAccounts1.borrowingVaults.publicKey,
            borrowingGlobalAccounts1.burningVault,
            pythPrices,
            2000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_repay_loan_incorrect_borrowing_vaults', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // deposit into borrowingMarketState2
        await newLoanee(env, borrowingGlobalAccounts2, stakingPoolAccounts2, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user sends borrowingMarketState2 borrowingVaults
        await expect(instructions_borrow.repayLoan(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts1.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts1.borrowingMarketState.publicKey,
            borrowingGlobalAccounts2.borrowingVaults.publicKey, // borrowingMarketState2 borrowingVaults
            borrowingGlobalAccounts1.burningVault,
            pythPrices,
            2000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_repay_loan_incorrect_burning_vault', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // create ATA for stablecoin
        const stablecoinAta = await setUpAssociatedStablecoinAccount(
            provider,
            user.publicKey,
            program.programId,
            borrowingGlobalAccounts.stablecoinMint,
            [user]
        );

        // user sends different burningVault
        await expect(instructions_borrow.repayLoan(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            stablecoinAta, // different burningVault
            pythPrices,
            2000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_repay_loan_incorrect_user_metadata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user1 sends user2's user metadata
        await expect(instructions_borrow.repayLoan(
            program,
            user1.publicKey,
            user2Accounts.userMetadata.publicKey, // user2 user metadata
            borrowingGlobalAccounts.stablecoinMint,
            user1Accounts.stablecoinAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            borrowingGlobalAccounts.burningVault,
            pythPrices,
            2000,
            [user1]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_repay_loan_incorrect_stablecoin_mint', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // create user stablecoin ATA for borrowingMarketState2 stablecoin
        const stablecoin2Ata = await setUpAssociatedStablecoinAccount(
            provider,
            user.publicKey,
            user.publicKey,
            borrowingGlobalAccounts2.stablecoinMint, // borrowingMarketState2 stablecoin mint
            [user]
        );
        await airdropStablecoin(program, initialMarketOwner, borrowingGlobalAccounts2.borrowingMarketState.publicKey,
            stablecoin2Ata, borrowingGlobalAccounts2.stablecoinMint, 2000);

        const { stablecoinMintAuthority: stablecoinMintAuthority2, } = await getBorrowingMarketState(program, borrowingGlobalAccounts2.borrowingMarketState.publicKey);
        const { burningVaultAuthority: burningVaultAuthority2, } = await getBorrowingVaults(program, borrowingGlobalAccounts2.borrowingVaults.publicKey);

        // user borrowingMarketState2 stablecoin mint and mint authority
        const ix = await program.instruction.repayLoan(
            new anchor.BN(2000),
            {
                accounts: instructions_borrow.utils.getRepayLoanAccounts(
                    user.publicKey,
                    userAccounts.userMetadata.publicKey,
                    borrowingGlobalAccounts1.borrowingMarketState.publicKey,
                    borrowingGlobalAccounts1.borrowingVaults.publicKey,
                    borrowingGlobalAccounts2.stablecoinMint, // borrowingMarketState2 mint
                    stablecoinMintAuthority2, // borrowingMarketState2 mint auth
                    borrowingGlobalAccounts2.burningVault, // borrowingMarketState2 burningVault
                    burningVaultAuthority2, // borrowingMarketState2 burningVaultAuthority
                    stablecoin2Ata, // borrowingMarketState2 stablecoin ATA
                ),
                signers: [user]
            });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, user.publicKey, [user]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_repay_loan_incorrect_borrower_stablecoin_ata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        const { stablecoinMintAuthority, } = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        const { burningVaultAuthority, } = await getBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);

        // user1 sends user2's stablecoin ATA, user2 co-signs
        const ix = await program.instruction.repayLoan(
            new anchor.BN(2000),
            {
                accounts: instructions_borrow.utils.getRepayLoanAccounts(
                    user1.publicKey,
                    user1Accounts.userMetadata.publicKey,
                    borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingGlobalAccounts.borrowingVaults.publicKey,
                    borrowingGlobalAccounts.stablecoinMint,
                    stablecoinMintAuthority,
                    borrowingGlobalAccounts.burningVault,
                    burningVaultAuthority,
                    user2Accounts.stablecoinAta, // user2 stablecoin ATA
                ),
                remainingAccounts: [
                    {
                        pubkey: user2.publicKey, // add user2 as signer
                        isWritable: true,
                        isSigner: true
                    }
                ],
                signers: [user1, user2,] // user2 co-signs
            });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, user1.publicKey, [user1, user2,]))
            .to.be.rejectedWith("0x8f"); // anchor raw constraint violation
    });

});
