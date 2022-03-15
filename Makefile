BORROWING_PROGRAM_ID=$(shell eval solana-keygen pubkey ./borrowing-keypair.json)
TESTWRITER_PROGRAM_ID=$(shell eval solana-keygen pubkey ./testwriter-keypair.json)
BORROWING_PROGRAM_DEPLOY_ACCOUNT=$(shell eval solana-keygen pubkey ./keypair.json)
include ./.env


.PHONY: build deploy build-client run listen deploy-new


build:
	mkdir -p ./target/deploy/
	cp borrowing-keypair.json ./target/deploy/
	cp testwriter-keypair.json ./target/deploy/
	anchor build -p borrowing --provider.cluster $(DEPLOYMENT_CLUSTER) --provider.wallet ./keypair.json
	anchor build -p testwriter --provider.cluster $(DEPLOYMENT_CLUSTER) --provider.wallet ./keypair.json

build-borrowing:
	mkdir -p ./target/deploy/
	cp borrowing-keypair.json ./target/deploy/
	anchor build -p borrowing --provider.cluster $(DEPLOYMENT_CLUSTER) --provider.wallet ./keypair.json

deploy-borrowing:
	echo $(BORROWING_PROGRAM_ID)
	anchor upgrade ./target/deploy/borrowing.so --program-id $(BORROWING_PROGRAM_ID) --provider.wallet ./keypair.json

# Pass file variable as the test file you want to run and test variable as the actual integration test you want to run 
refresh-borrowing: build-borrowing deploy-borrowing
	npx ts-mocha -t 1000000 tests/$(FILE).ts --grep $(TEST) 

# Only use this when you want to deploy the program at a new address (or for the first time)
# otherwise use the "deploy" to deploy to the old address
deploy-new:
	anchor deploy -p testwriter --provider.cluster $(DEPLOYMENT_CLUSTER) --provider.wallet ./keypair.json
	anchor deploy -p borrowing --provider.cluster $(DEPLOYMENT_CLUSTER) --provider.wallet ./keypair.json

# Use these whenever you already have a program id
deploy:
	echo $(BORROWING_PROGRAM_ID)
	echo $(TESTWRITER_PROGRAM_ID)
	anchor upgrade ./target/deploy/testwriter.so --program-id $(TESTWRITER_PROGRAM_ID) --provider.wallet ./keypair.json
	anchor upgrade ./target/deploy/borrowing.so --program-id $(BORROWING_PROGRAM_ID) --provider.wallet ./keypair.json

# Use this to run your own Serum DEX market on localnet
serum-swap-collateral:
	solana-test-validator -r --bpf-program 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin deps/serum_dex.so 

## Listen to on-chain logs
listen:
	solana logs $(BORROWING_PROGRAM_ID)

airdrop:
	solana airdrop 5 $(BORROWING_PROGRAM_DEPLOY_ACCOUNT) --url http://127.0.0.1:8899
