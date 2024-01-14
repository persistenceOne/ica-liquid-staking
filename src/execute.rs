use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, Response, SubMsg};
use persistence_std::types::{
    cosmos::base::v1beta1::Coin as StdCoin,
    ibc::applications::transfer::v1::{QueryDenomTraceRequest, QueryDenomTraceResponse},
    pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
};

use crate::{
    msg::{PresetIbcFee, Timeouts},
    state::{LSInfo, CURRENT_TX, IBC_CONFIG, LS_CONFIG},
    ContractError,
};

pub const DENOM_TRACE_QUERY_TYPE: &str = "/ibc.applications.transfer.v1.Query/DenomTrace";

pub(crate) const LS_REPLY_ID: u64 = 1u64;

pub fn try_liquid_staking(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver: Addr,
    transfer_channel: Option<String>,
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
        transfer_channel: transfer_channel.clone(),
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

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    active: Option<bool>,
    ls_prefix: Option<String>,
    preset_ibc_fee: Option<PresetIbcFee>,
    timeouts: Option<Timeouts>,
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

    // update ibc config
    let mut ibc_config = IBC_CONFIG.load(deps.storage)?;
    if let Some(preset_ibc_fee) = preset_ibc_fee {
        ibc_config.ibc_fee = preset_ibc_fee.clone().to_ibc_fee();

        res = res
            .add_attribute("ack_fee", preset_ibc_fee.ack_fee.to_string())
            .add_attribute("timeout_fee", preset_ibc_fee.timeout_fee.to_string());
    }
    if let Some(timeouts) = timeouts {
        ibc_config.ica_timeout = timeouts.ica_timeout;
        ibc_config.ibc_transfer_timeout = timeouts.ibc_transfer_timeout;

        res = res
            .add_attribute("ica_timeout", ibc_config.ica_timeout.to_string())
            .add_attribute(
                "ibc_transfer_timeout",
                ibc_config.ibc_transfer_timeout.to_string(),
            );
    }
    IBC_CONFIG.save(deps.storage, &ibc_config)?;

    Ok(res)
}
