require("dotenv").config();

import { PublicKey } from "@solana/web3.js";
import * as fs from "fs";
import { endpointFromCluster, pubkeyFromFile } from "./utils";

// Global Program Parameters
export const BORROWING_PROGRAM_ID = pubkeyFromFile("./borrowing-keypair.json");
export const TESTWRITER_PROGRAM_ID = pubkeyFromFile(
    "./testwriter-keypair.json"
);

export const DEX_PROGRAM_ID = new PublicKey(
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
);

export const BorrowingIdl = JSON.parse(
    fs.readFileSync("./target/idl/borrowing.json", "utf8")
);
export const TestWriterIdl = JSON.parse(
    fs.readFileSync("./target/idl/testwriter.json", "utf8")
);

export const env = {
    cluster: process.env.DEPLOYMENT_CLUSTER,
    endpoint: endpointFromCluster(process.env.DEPLOYMENT_CLUSTER),
};
