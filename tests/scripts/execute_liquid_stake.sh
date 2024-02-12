#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

contract_address=$(cat $METADATA/contract_address.txt)

echo "Executing liquid staking..."

receiver=$(persistenceCore keys show test2 -a)

msg=$(cat << EOF
{
    "liquid_stake": {
        "receiver": "$receiver"
    }
}
EOF
)

AMOUNT="100ibc/6AE2756AA7EAA8FA06E11472EA05CA681BD8D3FBC1AAA9F06C79D1EC1C90DC9B"

echo ">>> $PCORED tx wasm execute $contract_address $msg"
tx_hash=$($PCORED tx wasm execute $contract_address "$msg" --from $USER -y $GAS --amount $AMOUNT --chain-id $CHAIN_ID | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/ls_tx_hash.txt

sleep 5

contract_balance=$($PCORED q bank balances $contract_address | jq -r .balances)
echo "Contract Balance: $contract_balance"
echo "Tx Hash: $tx_hash, Contract Balance: $contract_balance" > $METADATA/contract_balance.txt
