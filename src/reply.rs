use cosmwasm_std::{ensure, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response, SubMsg};
use persistence_std::types::ibc::applications::transfer::v1::MsgTransfer;

use crate::{
    contract::TRANSFER_REPLY_ID,
    state::{CURRENT_TX, IBC_CONFIG},
    ContractError,
};

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: ls reply msg: {msg:?}").as_str());

    ensure!(
        msg.result.is_ok(),
        ContractError::SubcallError(msg.result.unwrap_err().to_string())
    );

    // load interim state
    let mut current_tx = CURRENT_TX.load(deps.storage)?;

    // get contract balance of ls asset
    let current_ls_token_balance = deps.querier.query_balance(
        env.contract.address.clone(),
        current_tx.ls_token_denom.clone(),
    )?;

    let balance_diff = current_ls_token_balance.amount - current_tx.prev_ls_token_balance;

    // update interim state
    current_tx.balance_change = balance_diff;
    CURRENT_TX.save(deps.storage, &current_tx)?;

    let mut res = Response::default().add_attribute("method", "handle_ls_reply");

    res = match current_tx.transfer_channel.clone() {
        Some(v) => {
            let ibc_config = IBC_CONFIG.load(deps.storage)?;

            // we define the persistence->host_chain timeout to be equal to:
            // current block + ICA timeout + ibc transfer timeout.
            // this assumes the worst possible time of delivery for the ICA message
            // which wraps the underlying MsgTransfer.
            let msg_transfer_timeout = env
                .block
                .time
                // we take the wrapping ICA tx timeout into account and assume the worst
                .plus_seconds(ibc_config.ica_timeout.u64())
                // and then add the preset ibc transfer timeout
                .plus_seconds(ibc_config.ibc_transfer_timeout.u64());

            // make ibc transfer
            let msg_transfer = MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: v.clone(),
                token: Some(
                    Coin {
                        denom: current_tx.ls_token_denom,
                        amount: balance_diff,
                    }
                    .into(),
                ),
                sender: env.contract.address.to_string(),
                receiver: current_tx.receiver.to_string(),
                timeout_height: None,
                timeout_timestamp: msg_transfer_timeout.nanos(),
                memo: "".to_string(),
            };

            let cosmos_msg = CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: msg_transfer.into(),
            };

            res.add_submessage(SubMsg::reply_on_success(cosmos_msg, TRANSFER_REPLY_ID))
                .add_attribute("transfer_channel", v)
        }
        None => {
            // send to receiver
            res.add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: current_tx.receiver.clone().to_string(),
                amount: vec![Coin {
                    denom: current_tx.ls_token_denom,
                    amount: balance_diff,
                }],
            }))
        }
    };

    Ok(res
        .add_attribute("minted_lst_amount", balance_diff.to_string())
        .add_attribute("receiver", current_tx.receiver.to_string()))
}

pub fn handle_transfer_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: transfer reply msg: {msg:?}").as_str());

    let res = Response::default().add_attribute("method", "handle_transfer_reply");

    if msg.result.is_ok() {
        // delete interim state
        CURRENT_TX.remove(deps.storage);

        return Ok(res);
    }

    // load interim state
    let current_tx = CURRENT_TX.load(deps.storage)?;

    // delete interim state
    CURRENT_TX.remove(deps.storage);

    let recovery_address = match current_tx.recovery_address {
        Some(v) => v,
        None => return Err(ContractError::InvalidRecoveryAddress {}),
    };

    Ok(res
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: recovery_address.clone().to_string(),
            amount: vec![Coin {
                denom: current_tx.ls_token_denom,
                amount: current_tx.balance_change,
            }],
        }))
        .add_attribute("minted_lst_amount", current_tx.balance_change.to_string())
        .add_attribute("receiver", current_tx.receiver.to_string())
        .add_attribute("sender", recovery_address.to_string()))
}
