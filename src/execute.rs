use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use persistence_std::types::{
    cosmos::base::v1beta1::Coin as StdCoin, pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
};

use crate::{
    state::{ASSETS, LS_CONFIG, STAKED_LIQUIDITY_INFO},
    ContractError,
};

pub fn try_liquid_staking(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver: Addr,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls execute");
    let config = LS_CONFIG.load(deps.storage)?;

    if config.active.is_none() || !config.active.unwrap() {
        return Err(ContractError::NotActive {});
    }

    let asset = ASSETS.load(deps.storage)?;

    let denom = info.funds[0].denom.clone();
    if denom != asset.native_asset_denom {
        return Err(ContractError::InvalidDenom {
            denom: denom.clone(),
            expected: asset.native_asset_denom,
        });
    }

    let amount = info.funds[0].amount.clone();
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    let msg_liquid_stake = MsgLiquidStake {
        amount: Some(StdCoin {
            denom: denom.clone(),
            amount: amount.to_string(),
        }),
        delegator_address: env.contract.address.to_string(),
    };

    let mut staked_liquidity_info = STAKED_LIQUIDITY_INFO.load(deps.storage)?;
    staked_liquidity_info.staked_amount_native += amount;
    STAKED_LIQUIDITY_INFO.save(deps.storage, &staked_liquidity_info)?;

    let res = Response::new()
        .add_message(CosmosMsg::Stargate {
            type_url: "/pstake.liquidstakeibc.v1beta1.MsgLiquidStake".to_string(),
            value: msg_liquid_stake.into(),
        })
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.clone().to_string(),
            amount: vec![Coin {
                denom: asset.ls_asset_denom,
                amount,
            }],
        }))
        .add_attribute("action", "liquid_stake")
        .add_attribute("amount", amount.to_string())
        .add_attribute("denom", denom)
        .add_attribute("receiver", receiver.to_string());
    Ok(res)
}
