import * as anchor from '@project-serum/anchor';
import * as operations_borrowing from "../operations_borrowing";
import { newLoanee } from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import { CollateralToken } from "../types";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { Transaction } from "@solana/web3.js";
import * as set_up from "../../src/set_up";
import { setUpAssociatedStablecoinAccount, setUpProgram } from "../../src/set_up";
import { getBorrowingMarketState } from "../data_provider";

chai.use(chaiAsPromised)

describe('tests_security_borrow_stablecoin', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    let pythPrices: any;
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

    it('security_borrow_stablecoin_incorrect_borrowing_market_state', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user sends borrowingMarketState2 borrowing vaults
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts1.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts2.borrowingMarketState.publicKey, // borrowingMarketState2 borrowing vaults
            borrowingGlobalAccounts1.borrowingVaults.publicKey,
            borrowingGlobalAccounts1.stakingPoolState.publicKey,
            borrowingGlobalAccounts1.borrowingFeesVault,
            stakingPoolAccounts1.treasuryVault,
            pythPrices,
            2000000000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_borrowing_vaults', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user sends borrowingMarketState2 borrowing vaults
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts1.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts1.borrowingMarketState.publicKey,
            borrowingGlobalAccounts2.borrowingVaults.publicKey, // borrowingMarketState2 borrowing vaults
            borrowingGlobalAccounts1.stakingPoolState.publicKey,
            borrowingGlobalAccounts1.borrowingFeesVault,
            stakingPoolAccounts1.treasuryVault,
            pythPrices,
            2000000000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_staking_pool_state', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user sends borrowingMarketState2 staking pool state
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            borrowingGlobalAccounts1.stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingGlobalAccounts1.borrowingMarketState.publicKey,
            borrowingGlobalAccounts1.borrowingVaults.publicKey,
            borrowingGlobalAccounts2.stakingPoolState.publicKey, // borrowingMarketState2 staking pool state
            borrowingGlobalAccounts1.borrowingFeesVault,
            stakingPoolAccounts1.treasuryVault,
            pythPrices,
            2000000000,
            [user]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_user_metadata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user1 sends user2's user metadata
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user1.publicKey,
            user2Accounts.userMetadata.publicKey, // user2 user metadata
            borrowingGlobalAccounts.stablecoinMint,
            user1Accounts.stablecoinAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            borrowingGlobalAccounts.stakingPoolState.publicKey,
            borrowingGlobalAccounts.borrowingFeesVault,
            stakingPoolAccounts.treasuryVault,
            pythPrices,
            2000000000,
            [user1]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_stablecoin_mint', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts2 = borrowingMarkets2.borrowingAccounts;

        const {
            borrower: user,
            borrowerAccounts: userAccounts,
            borrowerInitialBalance: userInitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
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

        const stablecoinMintAuthority2 = (await getBorrowingMarketState(program, borrowingGlobalAccounts2.borrowingMarketState.publicKey)).stablecoinMintAuthority;

        // user borrowingMarketState2 stablecoin mint and mint authority
        const ix = await program.instruction.borrowStablecoin(
            new anchor.BN(2000000000),
            {
                accounts: instructions_borrow.utils.getBorrowStablecoinAccounts(
                    user.publicKey,
                    userAccounts.userMetadata.publicKey,
                    borrowingGlobalAccounts2.stablecoinMint, // borrowingMarketState2 mint
                    stablecoinMintAuthority2, // borrowingMarketState2 mint auth
                    stablecoin2Ata, // borrowingMarketState2 stablecoin ATA
                    borrowingGlobalAccounts1.borrowingMarketState.publicKey,
                    borrowingGlobalAccounts1.borrowingVaults.publicKey,
                    borrowingGlobalAccounts1.stakingPoolState.publicKey,
                    borrowingGlobalAccounts2.borrowingFeesVault, // borrowingMarketState2 stablecoin fee account
                    stakingPoolAccounts1.treasuryVault,
                    pythPrices
                ),
                signers: [user]
            });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, user.publicKey, [user]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_borrow_stablecoin_incorrect_borrower_stablecoin_ata', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user1 sends user2's stablecoin ATA
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user1.publicKey,
            user1Accounts.userMetadata.publicKey,
            borrowingGlobalAccounts.stablecoinMint,
            user2Accounts.stablecoinAta, // user2 stablecoin ATA
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            borrowingGlobalAccounts.stakingPoolState.publicKey,
            borrowingGlobalAccounts.borrowingFeesVault,
            stakingPoolAccounts.treasuryVault,
            pythPrices,
            2000000000,
            [user1]
        )).to.be.rejectedWith("A raw constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_borrowing_fees_vault', async () => {
        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));
        const {
            borrower: user2,
            borrowerAccounts: user2Accounts,
            borrowerInitialBalance: user2InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // user1 sends user2's stablecoin ATA in place of borrowing fees vault
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user1.publicKey,
            user1Accounts.userMetadata.publicKey,
            borrowingGlobalAccounts.stablecoinMint,
            user1Accounts.stablecoinAta,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            borrowingGlobalAccounts.stakingPoolState.publicKey,
            user2Accounts.stablecoinAta, // user2 stablecoin ATA instead of fees vault
            stakingPoolAccounts.treasuryVault,
            pythPrices,
            2000000000,
            [user1]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });

    it('security_borrow_stablecoin_incorrect_treasury_vault', async () => {
        const borrowingMarkets1 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts1 = borrowingMarkets1.borrowingAccounts;
        const stakingPoolAccounts1 = borrowingMarkets1.stakingPoolAccounts;
        const borrowingMarkets2 = await operations_borrowing.initialiseBorrowingMarkets(env);
        const stakingPoolAccounts2 = borrowingMarkets2.stakingPoolAccounts;

        const {
            borrower: user1,
            borrowerAccounts: user1Accounts,
            borrowerInitialBalance: user1InitialBalance
        } = await newLoanee(env, borrowingGlobalAccounts1, stakingPoolAccounts1, pythPrices, 0, new Map<CollateralToken, number>([
            ["ETH", 12],
            ["SRM", 12],
        ]));

        // use stakingPoolAccounts2 treasury vault
        await expect(instructions_borrow.borrowStablecoin(
            program,
            user1.publicKey,
            user1Accounts.userMetadata.publicKey, // user2 user metadata
            borrowingGlobalAccounts1.stablecoinMint,
            user1Accounts.stablecoinAta,
            borrowingGlobalAccounts1.borrowingMarketState.publicKey,
            borrowingGlobalAccounts1.borrowingVaults.publicKey,
            borrowingGlobalAccounts1.stakingPoolState.publicKey,
            borrowingGlobalAccounts1.borrowingFeesVault,
            stakingPoolAccounts2.treasuryVault,
            pythPrices,
            2000000000,
            [user1]
        )).to.be.rejectedWith("A has_one constraint was violated");
    });
});
