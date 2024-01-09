use cosmwasm_std::{ensure, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response};

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

    Ok(Response::default()
        .add_attribute("method", "handle_ls_reply")
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: current_tx.receiver.clone().to_string(),
            amount: vec![Coin {
                denom: current_tx.ls_token_denom,
                amount: balance_diff,
            }],
        }))
        .add_attribute("sent_amount", balance_diff.to_string()))
}
