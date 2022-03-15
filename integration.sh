# set +x

# solana-keygen new -o /root/.config/solana/id.json --no-bip39-passphrase
solana config set -k ./keypair.json
cp keypair.json /root/.config/solana/id.json
solana config set --url http://localhost:8899/
if [ "$1" = "tests_swap" ]
then
    solana-test-validator -r --bpf-program 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin deps/serum_dex.so  > /dev/null 2>&1 &
else
    solana-test-validator > /dev/null 2>&1 &
fi
sleep 10
solana airdrop 5000 BSKmmWSyV42Pw3AwZHRFyiHpcBpQ3FyCYeHVecUanb6y
make build
make deploy-new

npx ts-mocha --parallel --jobs="$(find tests/** -type f -name 'tests_*.ts' | wc -l)" 'tests/**/tests_*.ts' --grep $1
