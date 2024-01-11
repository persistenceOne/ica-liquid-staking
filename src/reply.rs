use cosmwasm_std::{ensure, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response};
use persistence_std::types::ibc::applications::transfer::v1::MsgTransfer;

use crate::{state::CURRENT_TX, ContractError};

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls reply");

    ensure!(
        msg.result.is_ok(),
        ContractError::SubcallError(msg.result.unwrap_err().to_string())
    );

    // load interim state
    let current_tx = CURRENT_TX.load(deps.storage)?;

    // delete interim state
    CURRENT_TX.remove(deps.storage);

    // get contract balance of ls asset
    let current_ls_token_balance = deps.querier.query_balance(
        env.contract.address.clone(),
        current_tx.ls_token_denom.clone(),
    )?;

    let balance_diff = current_ls_token_balance.amount - current_tx.prev_ls_token_balance;

    let mut res = Response::default().add_attribute("method", "handle_ls_reply");

    let bank_send_msg = match current_tx.transfer_channel.clone() {
        Some(v) => {
            res = res.add_attribute("transfer_channel", v.clone());

            // make ibc transfer
            let msg_transfer = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: v,
                token: Some(
                    Coin {
                        denom: current_tx.ibc_denom,
                        amount: balance_diff,
                    }
                    .into(),
                ),
                sender: env.contract.address.to_string(),
                receiver: current_tx.receiver.to_string(),
                timeout_height: None,
                timeout_timestamp: 0,
                memo: "".to_string(),
            };
            CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: msg_transfer.into(),
            }
        }
        None => {
            // send to receiver
            CosmosMsg::Bank(BankMsg::Send {
                to_address: current_tx.receiver.clone().to_string(),
                amount: vec![Coin {
                    denom: current_tx.ls_token_denom,
                    amount: balance_diff,
                }],
            })
        }
    };

    Ok(res
        .add_message(bank_send_msg)
        .add_attribute("minted_lst_amount", balance_diff.to_string())
        .add_attribute("receiver", current_tx.receiver.to_string()))
}
