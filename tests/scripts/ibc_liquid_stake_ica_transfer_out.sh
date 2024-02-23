#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

contract_address=$(cat $METADATA/contract_address.txt)

echo "Executing liquid staking..."

receiver=$(persistenceCore keys show test2 -a)
memo=("{\"wasm\":{\"contract\":\"$contract_address\",\"msg\":{\"liquid_stake\":{\"receiver\":\"$receiver\"}}}}")

contract_balance=$($PCORED q bank balances $contract_address | jq -r .balances)
echo "Receiver Balance Before: $($PCORED q bank balances $receiver)"
echo "Contract Balance Before: $contract_balance"
echo "Contract Balance Before: $contract_balance" > $METADATA/contract_balance.txt

echo ">>> $OSMOSISD tx ibc-transfer transfer transfer channel-12 $contract_address 100uosmo --from test1 -y --memo $memo --gas auto --gas-adjustment 1.5 --fees 1000uosmo"
tx_hash=$($OSMOSISD tx ibc-transfer transfer transfer channel-12 $contract_address 100uosmo --from test1 -y --memo "$memo" --gas auto --gas-adjustment 1.5 --fees 1000uosmo | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/ibc_ls_tx_hash.txt

sleep 5

contract_balance=$($PCORED q bank balances $contract_address | jq -r .balances)
echo "Receiver Balance After: $($PCORED q bank balances $receiver)"
echo "Contract Balance After: $contract_balance"
echo "Contract Balance After: $contract_balance" >> $METADATA/contract_balance.txt
