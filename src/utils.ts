import * as anchor from "@project-serum/anchor";
import * as fs from "fs";
import * as serumCmn from "@project-serum/common";
import { sleep } from "@project-serum/common";
import { TokenInstructions } from "@project-serum/serum";

import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
    AccountInfo,
    ConfirmOptions,
    Connection,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    sendAndConfirmRawTransaction,
    Signer,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction,
    TransactionSignature,
} from "@solana/web3.js";
import { CollateralAmounts, CollateralToken, TokenMap } from "../tests/types";

const programPublicKey = "UpbA7oUWbQiXyvbkrMtfMF2gZ3W7F6U3jqxXbUvyPrD";
const programId = new anchor.web3.PublicKey(programPublicKey);

export const TROVE_DATA_SEED = "trove_data_7";
export const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID = new PublicKey(
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);

export async function findAssociatedTokenAddress(
    owner: PublicKey,
    tokenMintAddress: PublicKey
): Promise<PublicKey> {
    let res = (
        await PublicKey.findProgramAddress(
            [
                owner.toBuffer(),
                TOKEN_PROGRAM_ID.toBuffer(),
                tokenMintAddress.toBuffer(),
            ],
            SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
        )
    )[0];

    return res;
}

export async function executeTransaction(
    provider: anchor.Provider,
    ix: TransactionInstruction
) {
    const tx = new Transaction();
    tx.add(ix);

    let { blockhash } = await provider.connection.getRecentBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = provider.wallet.publicKey;

    let signed = await provider.wallet.signTransaction(tx);
    let txid = await provider.connection.sendRawTransaction(signed.serialize());
    let result = await provider.connection.confirmTransaction(txid);

    console.log(`Result ${JSON.stringify(result)}`);
}

export async function checkIfAccountExists(
    connection: Connection,
    account: PublicKey
): Promise<boolean> {
    const acc = await connection.getAccountInfo(account);
    return acc != null;
}

export async function createAssociatedTokenAccountIx(
    fundingAddress: PublicKey,
    walletAddress: PublicKey,
    splTokenMintAddress: PublicKey
): Promise<[TransactionInstruction, PublicKey]> {
    const associatedTokenAddress = await findAssociatedTokenAddress(
        walletAddress,
        splTokenMintAddress
    );
    const systemProgramId = new PublicKey("11111111111111111111111111111111");
    const keys = [
        {
            pubkey: fundingAddress,
            isSigner: true,
            isWritable: true,
        },
        {
            pubkey: associatedTokenAddress,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: walletAddress,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: splTokenMintAddress,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: systemProgramId,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: TokenInstructions.TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: SYSVAR_RENT_PUBKEY,
            isSigner: false,
            isWritable: false,
        },
    ];
    const ix = new TransactionInstruction({
        keys,
        programId: ASSOCIATED_TOKEN_PROGRAM_ID,
        data: Buffer.from([]),
    });
    return [ix, associatedTokenAddress];
}

export async function troveDataPubkey(userPubKey: PublicKey, seed: string) {
    // userPubKey is my SOLANA address
    // This function will *always* return the same value
    // This is essentially my metadata address

    let account = await PublicKey.createWithSeed(userPubKey, seed, programId);

    console.log(`Trove Account ${account}`);
    return account;
}

export async function createSolAccount(
    provider: anchor.Provider,
    programId: PublicKey
): Promise<PublicKey> {
    return (await serumCmn.createAccountRentExempt(provider, programId, 9))
        .publicKey;
}

export async function getMintOwnerAndNonce(marketPublicKey: PublicKey) {
    const nonce = new anchor.BN(0);

    while (nonce.toNumber() < 255) {
        try {
            const vaultOwner = await PublicKey.createProgramAddress(
                [marketPublicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
                programId
            );
            console.log(`Seed: [${marketPublicKey}]`);
            console.log(`Seed: [${nonce}]`);
            console.log(`Owner: [${vaultOwner}]`);
            return [vaultOwner, nonce];
        } catch (e) {
            nonce.iaddn(1);
        }
    }
    throw new Error("Unable to find nonce");
}

export async function createMint(
    provider: anchor.Provider,
    authority: PublicKey,
    decimals: number = 6
): Promise<PublicKey> {
    const mint = anchor.web3.Keypair.generate();
    const instructions = await createMintInstructions(
        provider,
        authority,
        mint.publicKey,
        decimals
    );

    const tx = new anchor.web3.Transaction();
    tx.add(...instructions);

    await provider.send(tx, [mint]);

    return mint.publicKey;
}

export async function mintTo(
    provider: anchor.Provider,
    mint: PublicKey,
    to: PublicKey,
    amount: number
): Promise<void> {
    const instruction = TokenInstructions.mintTo({
        mint,
        destination: to,
        amount,
        mintAuthority: provider.wallet.publicKey,
    });

    const tx = new anchor.web3.Transaction();
    tx.add(instruction);

    let sig = await provider.send(tx);
}

async function createMintInstructions(
    provider: anchor.Provider,
    authority: PublicKey,
    mint: PublicKey,
    decimals: number
): Promise<TransactionInstruction[]> {
    let instructions = [
        anchor.web3.SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: mint,
            space: 82,
            lamports: await provider.connection.getMinimumBalanceForRentExemption(82),
            programId: TOKEN_PROGRAM_ID,
        }),
        TokenInstructions.initializeMint({
            mint,
            decimals,
            mintAuthority: authority,
        }),
    ];
    return instructions;
}

export async function createTokenAccount(
    provider: anchor.Provider,
    mint: PublicKey,
    owner: PublicKey
): Promise<PublicKey> {
    const vault = anchor.web3.Keypair.generate();
    const tx = new anchor.web3.Transaction();
    tx.add(
        ...(await createTokenAccountInstrs(provider, vault.publicKey, mint, owner))
    );
    await provider.send(tx, [vault]);
    return vault.publicKey;
}

async function createTokenAccountInstrs(
    provider: anchor.Provider,
    newAccountPubkey: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    lamports?: number
) {
    if (lamports === undefined) {
        lamports = await provider.connection.getMinimumBalanceForRentExemption(165);
    }
    return [
        anchor.web3.SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey,
            space: 165,
            lamports,
            programId: TOKEN_PROGRAM_ID,
        }),
        TokenInstructions.initializeAccount({
            account: newAccountPubkey,
            mint,
            owner,
        }),
    ];
}

export async function getTokenAccount(
    provider: anchor.Provider,
    addr: PublicKey
) {
    return await serumCmn.getTokenAccount(provider, addr);
}

/**
 * Sends the given transaction, paid for and signed by the provider's wallet.
 *
 * @param tx      The transaction to send.
 * @param signers The set of signers in addition to the provdier wallet that
 *                will sign the transaction.
 * @param opts    Transaction confirmation options.
 */
export async function send(
    provider: anchor.Provider,
    tx: Transaction,
    payer: PublicKey,
    signers?: Array<Signer | undefined>,
    opts?: ConfirmOptions
): Promise<TransactionSignature> {
    if (signers === undefined) {
        signers = [];
    }

    let { blockhash } = await provider.connection.getRecentBlockhash();
    tx.feePayer = payer;
    tx.recentBlockhash = blockhash;

    // await provider.wallet.signTransaction(tx);
    signers.forEach((kp: Signer | undefined) => {
        if (kp !== undefined) {
            tx.partialSign(kp);
        }
    });

    const rawTx = tx.serialize();

    return await sendAndConfirmRawTransaction(provider.connection, rawTx, opts);
}

export async function buildAccountWithSeed(
    provider: anchor.Provider,
    programId: PublicKey,
    payer: PublicKey,
    signers: Array<Signer>,
    user: PublicKey,
    seed: string,
    space: number = 8
): Promise<PublicKey> {
    let account_public_key = await PublicKey.createWithSeed(
        user,
        seed,
        programId
    );

    const ix = SystemProgram.createAccountWithSeed({
        fromPubkey: payer,
        newAccountPubkey: account_public_key,
        basePubkey: user,
        seed: seed,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(
            space,
            "singleGossip"
        ),
        space: space,
        programId: programId,
    });

    if (
        (await checkIfAccountExists(provider.connection, account_public_key)) ==
        true
    ) {
        return account_public_key;
    }

    if (signers.length === 0) {
        await executeTransaction(provider, ix);
    } else {
        const tx = new Transaction();
        tx.add(ix);

        await send(provider, tx, payer, signers);
    }

    return account_public_key;
}

export async function solAccountWithMinBalance(
    provider: anchor.Provider,
    minSolBalance: number
): Promise<{ keyPair: Keypair; account: AccountInfo<Buffer> }> {
    const keyPair = anchor.web3.Keypair.generate();
    let solAccount = await solAirdropMin(
        provider,
        keyPair.publicKey,
        minSolBalance
    );
    return { keyPair, account: solAccount };
}

export async function solAirdropMin(provider: anchor.Provider, account: PublicKey, minSolAirdrop: number): Promise<AccountInfo<Buffer>> {
    console.log("New account with SOL Balance", minSolAirdrop);
    const airdropBatchAmount = Math.max(5, minSolAirdrop);
    let solAccount = await provider.connection.getAccountInfo(account);
    let currentBalance: number | undefined = lamportsToColl(solAccount?.lamports, "SOL");
    while (lamportsToColl(solAccount?.lamports, "SOL") < minSolAirdrop) {
        try {
            let res = await provider.connection.requestAirdrop(account, collToLamports(airdropBatchAmount, "SOL"));
        } catch (e) {
            await sleep(100);
            console.log("Error", e);
        }
        await sleep(100);
        solAccount = await provider.connection.getAccountInfo(account);
        currentBalance = solAccount?.lamports;
    }
    if (solAccount === null) {
        throw new Error(`SOL Account '${account}' not found`);
    }
    return solAccount;
}

export const FACTOR = 1_000_000.0;
export function decimalToU64(n: number): number {
    let n1 = n * FACTOR;
    let n2 = Math.trunc(n1);
    return n2;
}

export function u64ToDecimal(n: number): number {
    let n1 = n / FACTOR;
    return n1;
}

export function lamportsToColl(
    lamports: number | undefined,
    token: CollateralToken
): number {
    let factor = LAMPORTS_PER_SOL;
    switch (token) {
        case "SOL": {
            factor = LAMPORTS_PER_SOL;
            break;
        }
        case "ETH": {
            factor = FACTOR;
            break;
        }
        case "BTC": {
            factor = FACTOR;
            break;
        }
        case "SRM": {
            factor = FACTOR;
            break;
        }
        case "RAY": {
            factor = FACTOR;
            break;
        }
        case "FTT": {
            factor = FACTOR;
            break;
        }
    }

    if (lamports != null) {
        if (lamports === 0) {
            return 0;
        }
        return lamports / factor;
    } else {
        return -1;
    }
}

export function collToLamports(amount: number, token: CollateralToken): number {
    let factor = LAMPORTS_PER_SOL;
    switch (token) {
        case "SOL": {
            factor = LAMPORTS_PER_SOL;
            break;
        }
        case "ETH": {
            factor = FACTOR;
            break;
        }
        case "BTC": {
            factor = FACTOR;
            break;
        }
        case "SRM": {
            factor = FACTOR;
            break;
        }
        case "RAY": {
            factor = FACTOR;
            break;
        }
        case "FTT": {
            factor = FACTOR;
            break;
        }
    }

    return amount * factor;
}

export function pubkeyFromFile(filepath: string): PublicKey {
    const fileContents = fs.readFileSync(filepath, "utf8");
    const privateArray = fileContents
        .replace("[", "")
        .replace("]", "")
        .split(",")
        .map(function (item) {
            return parseInt(item, 10);
        });
    const array = Uint8Array.from(privateArray);
    const keypair = Keypair.fromSecretKey(array);
    return keypair.publicKey;
}

export function endpointFromCluster(cluster: string | undefined): string {
    console.log("Cluster", cluster);
    switch (cluster) {
        case "devnet":
            return "https://api.devnet.solana.com/";
        // case "devnet": return "https://dark-lingering-snow.solana-devnet.quiknode.pro/af2e96524464a0ccb0d1a16f1f017033c5808210/";
        case "localnet":
            return "http://127.0.0.1:8899";
    }
    return "err";
}

export const mapToObj = (m: Map<any, any>): any => {
    return Array.from(m).reduce((obj, [key, value]) => {
        obj[key] = value;
        return obj;
    }, {} as any);
};

export const tokenMapPrint = (m: TokenMap): any => {
    return Object.entries(m).reduce((obj, [key, value]) => {
        if (!value.isZero()) {
            obj[key.toUpperCase()] = lamportsToColl(
                value.toNumber(),
                key.toUpperCase() as CollateralToken
            );
        }
        return obj;
    }, {} as any);
};

export const collateralMapPrint = (m: CollateralAmounts): any => {
    return Object.entries(m).reduce((obj, [key, value]) => {
        if (value !== 0) {
            obj[key.toUpperCase()] = lamportsToColl(value, key.toUpperCase() as CollateralToken);
        }
        return obj;
    }, {} as any);
};

export const publicKeyReplacer = (_: string, value: any) => {
    if (value._bn) {
        return value.toBase58();
    }
    return value;
};

export const publicKeyReviver = (_: string, value: any) => {
    if (typeof value === "string") {
        return new PublicKey(value);
    }
    return value;
};

/**
 *
 * Maps the private Anchor type ProgramError to a normal Error.
 * Pass ProgramErr.msg as the Error message so that it can be used with chai matchers
 *
 * @param fn - function which may throw an anchor ProgramError
 */
export async function mapAnchorError<T>(fn: Promise<T>): Promise<T> {
    try {
        return await fn;
    } catch (e: any) {
        if (e.constructor.name === "ProgramError") {
            throw new Error(e.msg);
        }
        throw e;
    }
}

export async function createAccountIx(
    provider: anchor.Provider,
    account: PublicKey,
    space: number,
    signer: Signer,
    programId: PublicKey
): Promise<TransactionInstruction> {
    return SystemProgram.createAccount({
        newAccountPubkey: account,
        fromPubkey: signer.publicKey,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(
            space
        ),
        space,
        programId,
    });
}