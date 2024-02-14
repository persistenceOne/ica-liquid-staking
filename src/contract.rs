#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};

use crate::{
    error::ContractError,
    execute::{try_claim, try_liquid_staking, update_config},
    msg::{ExecuteMsg, IbcConfig, InstantiateMsg, LsConfig, QueryMsg},
    query,
    reply::{handle_ls_reply, handle_transfer_reply},
    state::{IBC_CONFIG, LS_CONFIG},
};

pub const LS_REPLY_ID: u64 = 1;
pub const TRANSFER_REPLY_ID: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls instantiate");

    let ls_config = LsConfig {
        admin: info.sender.clone(),
        active: true,
        ls_prefix: msg.ls_prefix.clone(),
    };
    LS_CONFIG.save(deps.storage, &ls_config)?;

    let timeouts = msg.timeouts.unwrap_or_default();

    // ibc fees and timeouts
    IBC_CONFIG.save(
        deps.storage,
        &IbcConfig {
            ibc_transfer_timeout: timeouts.ibc_transfer_timeout,
            ica_timeout: timeouts.ica_timeout,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("active", "true")
        .add_attribute("ls_prefix", msg.ls_prefix)
        .add_attribute("ica_timeout", timeouts.ica_timeout)
        .add_attribute("ibc_transfer_timeout", timeouts.ibc_transfer_timeout))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::LiquidStake {
            receiver,
            transfer_channel,
        } => try_liquid_staking(deps, env, info, receiver, transfer_channel),

        ExecuteMsg::UpdateConfig {
            active,
            ls_prefix,
            timeouts,
        } => update_config(deps, env, info, active, ls_prefix, timeouts),

        ExecuteMsg::Claim {} => try_claim(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        LS_REPLY_ID => handle_ls_reply(deps, env, msg),
        TRANSFER_REPLY_ID => handle_transfer_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LsConfig {} => to_json_binary(&query::query_ls_config(deps)?),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::execute::DENOM_TRACE_QUERY_TYPE;
    use crate::msg::Timeouts;
    use crate::state::{LSInfo, CURRENT_TX};

    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
    use cosmwasm_std::{
        attr, coins, from_json, Addr, BalanceResponse, BankMsg, BankQuery, Coin, ContractResult,
        CosmosMsg, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, ReplyOn, SubMsg,
        SubMsgResponse, SystemError, SystemResult, Uint128, Uint64,
    };
    use persistence_std::types::ibc::applications::transfer::v1::MsgTransfer;
    use persistence_std::types::{
        cosmos::base::v1beta1::Coin as StdCoin,
        ibc::applications::transfer::v1::{
            DenomTrace, QueryDenomTraceRequest, QueryDenomTraceResponse,
        },
        pstake::liquidstakeibc::v1beta1::MsgLiquidStake,
    };

    use prost::Message;

    const NATIVE_IBC_DENOM: &str =
        "ibc/C8A74ABBE2AF892E15680D916A7C22130585CE5704F9B17A10F184A90D53BECA";
    const NATIVE_BASE_DENOM: &str = "uatom";
    const LIQUIDSTAKE_DENOM: &str = "stk/uatom";

    pub struct WasmMockQuerier {
        pub denom_trace: HashMap<String, QueryDenomTraceResponse>,
    }

    // Implements the Querier trait to be used as a MockQuery object
    impl Querier for WasmMockQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<Empty> = match from_json(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return SystemResult::Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                }
            };
            self.handle_query(&request)
        }
    }

    impl WasmMockQuerier {
        pub fn new() -> Self {
            WasmMockQuerier {
                denom_trace: HashMap::new(),
            }
        }

        fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match request {
                QueryRequest::Stargate { path, data } => {
                    if path == DENOM_TRACE_QUERY_TYPE {
                        let query_denom_trace_request =
                            QueryDenomTraceRequest::decode(data.as_slice()).unwrap();
                        match self.denom_trace.get(&query_denom_trace_request.hash) {
                            Some(resp) => SystemResult::Ok(to_json_binary(&resp).into()),
                            None => SystemResult::Err(SystemError::Unknown {}),
                        }
                    } else {
                        panic!("Mocked query not supported for stargate path {}", path);
                    }
                }
                QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
                    if address == &Addr::unchecked("cosmos2contract")
                        && denom == &LIQUIDSTAKE_DENOM.to_string()
                    {
                        let bank_res = BalanceResponse {
                            amount: Coin {
                                amount: Uint128::new(2000u128),
                                denom: denom.to_string(),
                            },
                        };
                        SystemResult::Ok(ContractResult::from(to_json_binary(&bank_res)))
                    } else {
                        unimplemented!()
                    }
                }
                _ => panic!("DO NOT ENTER HERE"),
            }
        }

        pub fn mock_denom_trace(&mut self, ibc_hash: String) {
            self.denom_trace.insert(
                ibc_hash,
                QueryDenomTraceResponse {
                    denom_trace: Some(DenomTrace {
                        path: "transfer/channel-0".to_string(),
                        base_denom: NATIVE_BASE_DENOM.to_string(),
                    }),
                },
            );
        }
    }

    // Helper function to instantiate the contract
    fn default_instantiate() -> (
        OwnedDeps<MockStorage, MockApi, WasmMockQuerier, Empty>,
        Env,
        MessageInfo,
    ) {
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: Default::default(),
        };

        let msg = InstantiateMsg {
            ls_prefix: "stk/".to_string(),
            timeouts: None,
        };

        let resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            resp.attributes,
            vec![
                attr("method", "instantiate"),
                attr("owner", "creator"),
                attr("active", "true"),
                attr("ls_prefix", "stk/"),
                attr("ica_timeout", "18000"),
                attr("ibc_transfer_timeout", "18000"),
            ]
        );

        // Mock each pool in the querier
        deps.querier.mock_denom_trace(NATIVE_IBC_DENOM.to_string());

        (deps, env, info)
    }

    #[test]
    fn proper_initialization() {
        let (deps, _env, _info) = default_instantiate();

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LsConfig {}).unwrap();
        let value: LsConfig = from_json(&res).unwrap();
        assert_eq!(true, value.active);
    }

    #[test]
    fn update_config() {
        let (mut deps, _env, info) = default_instantiate();

        let msg = ExecuteMsg::UpdateConfig {
            active: Some(false),
            ls_prefix: Some("newprefix/".to_string()),
            timeouts: Some(Timeouts {
                ica_timeout: Uint64::new(10000u64),
                ibc_transfer_timeout: Uint64::new(10000u64),
            }),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "update_config"),
                attr("active", "false"),
                attr("ls_prefix", "newprefix/"),
                attr("ica_timeout", "10000"),
                attr("ibc_transfer_timeout", "10000"),
            ]
        );

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LsConfig {}).unwrap();
        let value: LsConfig = from_json(&res).unwrap();
        assert_eq!(false, value.active);
        assert_eq!("newprefix/", value.ls_prefix);
    }

    #[test]
    fn liquid_stake() {
        let (mut deps, _env, _info) = default_instantiate();

        let deposit_amount = Uint128::from(2000u128);

        // beneficiary can release it
        let info = mock_info("anyone", &coins(deposit_amount.into(), NATIVE_IBC_DENOM));
        let msg = ExecuteMsg::LiquidStake {
            receiver: Addr::unchecked("receiver"),
            transfer_channel: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 1,
                msg: CosmosMsg::Stargate {
                    type_url: "/pstake.liquidstakeibc.v1beta1.MsgLiquidStake".to_string(),
                    value: MsgLiquidStake {
                        amount: Some(StdCoin {
                            denom: NATIVE_IBC_DENOM.to_string(),
                            amount: deposit_amount.to_string(),
                        }),
                        delegator_address: "cosmos2contract".to_string(),
                    }
                    .into(),
                },
                gas_limit: None,
                reply_on: ReplyOn::Success
            }
        );

        // ensure attributes are set
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "liquid_stake"),
                attr("sender", "anyone"),
                attr("native_amount", deposit_amount.to_string()),
                attr("native_ibc_denom", NATIVE_IBC_DENOM),
                attr("native_base_denom", NATIVE_BASE_DENOM),
                attr("ls_token_denom", LIQUIDSTAKE_DENOM),
                attr("receiver", "receiver"),
            ]
        );
    }

    #[test]
    fn handle_ls_reply_with_ica_transfer_out() {
        let (mut deps, _env, _info) = default_instantiate();

        let msg = Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(to_json_binary("response").unwrap()),
            }),
        };

        let current_tx = LSInfo {
            sender: Addr::unchecked("sender"),
            receiver: Addr::unchecked("receiver"),
            transfer_channel: None,
            ibc_denom: NATIVE_IBC_DENOM.to_string(),
            ls_token_denom: LIQUIDSTAKE_DENOM.to_string(),
            prev_ls_token_balance: Uint128::new(1000u128),
            balance_change: Uint128::new(1000u128),
        };
        CURRENT_TX.save(deps.as_mut().storage, &current_tx).unwrap();

        let res = handle_ls_reply(deps.as_mut(), mock_env(), msg).unwrap();

        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 2,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "receiver".to_string(),
                    amount: vec![Coin {
                        denom: LIQUIDSTAKE_DENOM.to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                }),
                gas_limit: None,
                reply_on: ReplyOn::Success
            }
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "handle_ls_reply"),
                attr("minted_lst_amount", Uint128::new(1000u128).to_string()),
                attr("receiver", "receiver")
            ]
        );
    }

    #[test]
    fn handle_ls_reply_with_ibc_transfer_out() {
        let (mut deps, env, _info) = default_instantiate();

        let msg = Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(to_json_binary("response").unwrap()),
            }),
        };

        let current_tx = LSInfo {
            sender: Addr::unchecked("sender"),
            receiver: Addr::unchecked("ibcreceiver"),
            transfer_channel: Some("channel-0".to_string()),
            ibc_denom: NATIVE_IBC_DENOM.to_string(),
            ls_token_denom: LIQUIDSTAKE_DENOM.to_string(),
            prev_ls_token_balance: Uint128::new(1000u128),
            balance_change: Uint128::new(1000u128),
        };
        CURRENT_TX.save(deps.as_mut().storage, &current_tx).unwrap();

        let res = handle_ls_reply(deps.as_mut(), mock_env(), msg).unwrap();

        let msg_transfer_timeout = env.block.time.plus_seconds(18000).plus_seconds(18000);

        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 2,
                msg: CosmosMsg::Stargate {
                    type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                    value: MsgTransfer {
                        source_port: "transfer".to_string(),
                        source_channel: "channel-0".to_string(),
                        token: Some(
                            Coin {
                                denom: LIQUIDSTAKE_DENOM.to_string(),
                                amount: Uint128::new(1000u128),
                            }
                            .into()
                        ),
                        sender: "cosmos2contract".to_string(),
                        receiver: "ibcreceiver".to_string(),
                        timeout_height: None,
                        timeout_timestamp: msg_transfer_timeout.nanos(),
                        memo: "".to_string(),
                    }
                    .into(),
                },
                gas_limit: None,
                reply_on: ReplyOn::Success
            }
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "handle_ls_reply"),
                attr("transfer_channel", "channel-0"),
                attr("minted_lst_amount", Uint128::new(1000u128).to_string()),
                attr("receiver", "ibcreceiver")
            ]
        );
    }

    #[test]
    fn handle_transfer_reply_with_ibc_transfer_out() {
        let (mut deps, _env, _info) = default_instantiate();

        let msg = Reply {
            id: 2,
            result: cosmwasm_std::SubMsgResult::Err("error".to_string()),
        };

        let current_tx = LSInfo {
            sender: Addr::unchecked("sender"),
            receiver: Addr::unchecked("ibcreceiver"),
            transfer_channel: Some("channel-0".to_string()),
            ibc_denom: NATIVE_IBC_DENOM.to_string(),
            ls_token_denom: LIQUIDSTAKE_DENOM.to_string(),
            prev_ls_token_balance: Uint128::new(1000u128),
            balance_change: Uint128::new(1000u128),
        };
        CURRENT_TX.save(deps.as_mut().storage, &current_tx).unwrap();

        let res = handle_transfer_reply(deps.as_mut(), mock_env(), msg).unwrap();

        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 0,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "sender".to_string(),
                    amount: vec![Coin {
                        denom: LIQUIDSTAKE_DENOM.to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                }),
                gas_limit: None,
                reply_on: ReplyOn::Never
            }
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "handle_transfer_reply"),
                attr("minted_lst_amount", Uint128::new(1000u128).to_string()),
                attr("receiver", "ibcreceiver"),
                attr("sender", "sender")
            ]
        );
    }
}
