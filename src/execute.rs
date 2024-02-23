use cosmwasm_std::{
    Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, Response, SubMsg,
};
use persistence_std::types::{
    cosmos::base::v1beta1::Coin as StdCoin,
    ibc::applications::transfer::v1::{QueryDenomTraceRequest, QueryDenomTraceResponse},
    pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
};

use crate::{
    contract::LS_REPLY_ID,
    state::{LSInfo, CURRENT_TX, LS_CONFIG},
    ContractError,
};

pub const DENOM_TRACE_QUERY_TYPE: &str = "/ibc.applications.transfer.v1.Query/DenomTrace";
const PERSISTENCE_ADDRESS_PREFIX: &str = "persistence";

pub fn try_liquid_staking(
    deps: DepsMut,
    env: Env,
    coin: Coin,
    sender: Addr,
    mut receiver: Addr,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls execute");

    let config = LS_CONFIG.load(deps.storage)?;
    if !config.active {
        return Err(ContractError::NotActive {});
    }

    // validate receiver address
    receiver = match deps.api.addr_validate(receiver.as_str()) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::InvalidReceiverAddress {
                receiver: receiver.to_string(),
            });
        }
    };

    // receiver address must have prefix "persistence"
    if !receiver.to_string().starts_with(PERSISTENCE_ADDRESS_PREFIX) {
        return Err(ContractError::InvalidReceiverAddress {
            receiver: receiver.to_string(),
        });
    }

    let native_ibc_denom = coin.denom.clone();
    let native_amount = coin.amount;

    // get base denom by querying denom trace
    let query_denom_trace_request = QueryDenomTraceRequest {
        hash: native_ibc_denom.clone(),
    };
    let query_denom_trace_response: QueryDenomTraceResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: DENOM_TRACE_QUERY_TYPE.to_string(),
            data: query_denom_trace_request.into(),
        })?;

    let native_base_denom = match query_denom_trace_response.denom_trace {
        Some(denom_trace) => denom_trace.base_denom,
        None => {
            return Err(ContractError::InvalidDenom {
                denom: native_ibc_denom,
            });
        }
    };

    // get ls token denom
    let ls_token_denom = format!("{}{}", config.ls_prefix, native_base_denom);

    // get contract balance of ls asset
    let contract_ls_token_balance = deps
        .querier
        .query_balance(env.contract.address.clone(), ls_token_denom.clone())?;

    // save interim state
    let current_tx = LSInfo {
        receiver: receiver.clone(),
        ls_token_denom: ls_token_denom.clone(),
        prev_ls_token_balance: contract_ls_token_balance.amount,
    };
    CURRENT_TX.save(deps.storage, &current_tx)?;

    // create the message for liquid staking
    let msg_liquid_stake = MsgLiquidStake {
        amount: Some(StdCoin {
            denom: native_ibc_denom.clone(),
            amount: native_amount.to_string(),
        }),
        delegator_address: env.contract.address.to_string(),
    };

    let res = Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Stargate {
                type_url: "/pstake.liquidstakeibc.v1beta1.MsgLiquidStake".to_string(),
                value: msg_liquid_stake.into(),
            },
            LS_REPLY_ID,
        ))
        .add_attribute("action", "liquid_stake")
        .add_attribute("sender", sender.to_string())
        .add_attribute("native_amount", native_amount.to_string())
        .add_attribute("native_ibc_denom", native_ibc_denom)
        .add_attribute("native_base_denom", native_base_denom)
        .add_attribute("ls_token_denom", ls_token_denom)
        .add_attribute("receiver", receiver.to_string());
    Ok(res)
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    active: Option<bool>,
    ls_prefix: Option<String>,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: update config");

    let mut ls_config = LS_CONFIG.load(deps.storage)?;

    // only admin can update config
    if info.sender != ls_config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut res = Response::new().add_attribute("method", "update_config");

    // update contract config
    if let Some(active) = active {
        ls_config.active = active;

        res = res.add_attribute("active", ls_config.active.to_string());
    }
    if let Some(ls_prefix) = ls_prefix {
        ls_config.ls_prefix = ls_prefix;

        res = res.add_attribute("ls_prefix", ls_config.clone().ls_prefix);
    }
    LS_CONFIG.save(deps.storage, &ls_config)?;

    Ok(res)
}
