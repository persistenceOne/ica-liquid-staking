use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, Response, SubMsg};
use persistence_std::types::{
    cosmos::base::v1beta1::Coin as StdCoin, pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
};

use osmosis_std::types::ibc::applications::transfer::v1::{
    QueryDenomTraceRequest, QueryDenomTraceResponse,
};

use crate::{
    state::{LSInfo, CURRENT_TX, LS_CONFIG},
    ContractError,
};

pub const DENOM_TRACE_QUERY_TYPE: &str = "/ibc.applications.transfer.v1.Query/DenomTrace";

pub(crate) const LS_REPLY_ID: u64 = 1u64;

pub fn try_liquid_staking(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver: Addr,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls execute");

    let config = LS_CONFIG.load(deps.storage)?;
    if !config.active {
        return Err(ContractError::NotActive {});
    }

    let native_ibc_denom = info.funds[0].denom.clone();
    let native_amount = info.funds[0].amount;

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
        ibc_denom: native_ibc_denom.clone(),
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
        .add_attribute("native_amount", native_amount.to_string())
        .add_attribute("native_ibc_denom", native_ibc_denom)
        .add_attribute("native_base_denom", native_base_denom)
        .add_attribute("ls_token_denom", ls_token_denom)
        .add_attribute("receiver", receiver.to_string());
    Ok(res)
}
