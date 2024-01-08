use std::str::FromStr;

use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, QueryRequest, Response, SubMsg, Uint128,
};
use cw_utils::must_pay;
use persistence_std::types::{
    cosmos::base::v1beta1::Coin as StdCoin,
    pstake::liquidstakeibc::v1beta1::{
        MsgLiquidStake, QueryExchangeRateRequest, QueryExchangeRateResponse,
    },
};

use crate::{
    state::{LSInfo, ASSETS, CURRENT_TX, LS_CONFIG, STAKED_LIQUIDITY_INFO},
    ContractError,
};

pub const LIQUIDSTAKEIBC_RATE_QUERY_TYPE: &str =
    "/pstake.liquidstakeibc.v1beta1.Query/ExchangeRate";

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

    let asset = ASSETS.load(deps.storage)?;
    let denom = asset.native_asset_denom;

    // get contract balance of ls asset
    let contract_lst_balance = deps
        .querier
        .query_balance(env.contract.address.clone(), asset.ls_asset_denom)?;

    // save interim state
    let current_tx = LSInfo {
        receiver: receiver.clone(),
        prev_ls_token_balance: contract_lst_balance.amount,
    };
    CURRENT_TX.save(deps.storage, &current_tx)?;

    // check if the denom and amount is valid
    let native_amount: Uint128 = match must_pay(&info, &denom) {
        Ok(coin_amount) => coin_amount,
        Err(e) => {
            return Err(ContractError::PaymentError(e.to_string()));
        }
    };

    // create the message
    let msg_liquid_stake = MsgLiquidStake {
        amount: Some(StdCoin {
            denom: denom.clone(),
            amount: native_amount.to_string(),
        }),
        delegator_address: env.contract.address.to_string(),
    };

    // query exchange rate
    let q = QueryExchangeRateRequest {
        chain_id: config.chain_id,
    };
    let exchange_rate_response: QueryExchangeRateResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: LIQUIDSTAKEIBC_RATE_QUERY_TYPE.to_string(),
            data: q.into(),
        })?;
    let exchange_rate = exchange_rate_response.rate;

    // calculate staked amount to be sent to the receiver
    let exchange_rate_decimal = Decimal::from_str(&exchange_rate)?;
    let amount_decimal = Decimal::from_str(&native_amount.to_string())?;
    let lst_mint_amount_decimal = amount_decimal.checked_mul(exchange_rate_decimal)?;

    // convert decimal to Uint128 to be sent to the receiver and
    let lst_mint_amount = Decimal::to_uint_floor(lst_mint_amount_decimal);

    // update the staked amount
    let mut staked_liquidity_info = STAKED_LIQUIDITY_INFO.load(deps.storage)?;
    staked_liquidity_info.staked_amount_native += native_amount;
    STAKED_LIQUIDITY_INFO.save(deps.storage, &staked_liquidity_info)?;

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
        .add_attribute("lst_mint_amount", lst_mint_amount.to_string())
        .add_attribute("exchange_rate", exchange_rate)
        .add_attribute("denom", denom)
        .add_attribute("receiver", receiver.to_string());
    Ok(res)
}
