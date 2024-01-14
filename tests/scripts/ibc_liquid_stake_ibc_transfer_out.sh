#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

contract_address=$(cat $METADATA/contract_address.txt)

echo "Executing liquid staking..."

receiver=$($GAIAD keys show test2 -a)
memo=("{\"wasm\":{\"contract\":\"$contract_address\",\"msg\":{\"liquid_stake\":{\"receiver\":\"$receiver\",\"transfer_channel\":\"channel-0\"}}}}")

contract_balance=$($PCORED q bank balances $contract_address | jq -r .balances)
echo "Receiver Balance Before: $($GAIAD q bank balances $receiver)"
echo "Contract Balance Before: $contract_balance"
echo "Contract Balance Before: $contract_balance" > $METADATA/contract_balance.txt

echo ">>> $GAIAD tx ibc-transfer transfer transfer channel-0 $contract_address 100uatom --from test1 -y --memo $memo --gas auto --gas-adjustment 1.5 --fees 1000uatom"
tx_hash=$($GAIAD tx ibc-transfer transfer transfer channel-0 $contract_address 100uatom --from test1 -y --memo "$memo" --gas auto --gas-adjustment 1.5 --fees 1000uatom | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/ibc_ls_tx_hash.txt

sleep 5

contract_balance=$($PCORED q bank balances $contract_address | jq -r .balances)
echo "Receiver Balance After: $($GAIAD q bank balances $receiver)"
echo "Contract Balance After: $contract_balance"
echo "Contract Balance After: $contract_balance" >> $METADATA/contract_balance.txt
