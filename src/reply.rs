use cosmwasm_std::{ensure, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response};

use crate::{state::CURRENT_TX, ContractError};

pub fn handle_ls_reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    deps.api
        .debug(format!("WASMDEBUG: ls reply msg: {msg:?}").as_str());

    ensure!(
        msg.result.is_ok(),
        ContractError::SubcallError(msg.result.unwrap_err().to_string())
    );

    // load interim state
    let current_tx = CURRENT_TX.load(deps.storage)?;

    // get contract balance of ls asset
    let current_ls_token_balance = deps.querier.query_balance(
        env.contract.address.clone(),
        current_tx.ls_token_denom.clone(),
    )?;

    let balance_diff = current_ls_token_balance.amount - current_tx.prev_ls_token_balance;

    let res = Response::default()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: current_tx.receiver.clone().to_string(),
            amount: vec![Coin {
                denom: current_tx.ls_token_denom,
                amount: balance_diff,
            }],
        }))
        .add_attribute("method", "handle_ls_reply")
        .add_attribute("minted_lst_amount", balance_diff.to_string())
        .add_attribute("receiver", current_tx.receiver.to_string());

    Ok(res)
}
