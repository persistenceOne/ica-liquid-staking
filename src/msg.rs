use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint64};

pub const DEFAULT_TIMEOUT: u64 = 60 * 60 * 5; // 5 hours

#[cw_serde]
pub struct InstantiateMsg {
    /// LS token prefix used to identify LS tokens
    /// e.g. "stk/"
    pub ls_prefix: String,
    /// timeouts for IBC transfers
    /// ica_timeout is the timeout for the IBC channel
    /// ibc_transfer_timeout is the timeout for the IBC transfer
    /// both timeouts are in seconds
    /// if not set, default values will be used
    /// default values are 5 hours
    pub timeouts: Option<Timeouts>,
}

#[cw_serde]
pub struct LsConfig {
    /// admin address
    pub admin: Addr,
    /// Flag to enable/disable the contract
    pub active: bool,
    /// LS token prefix
    pub ls_prefix: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Liquid stake tokens
    LiquidStake {
        /// Receiver of the liquid staked tokens
        /// If `transfer_channel` is set, then `receiver` must be an IBC address
        receiver: Addr,
        /// IBC transfer channel that allow to optionally specify
        /// an IBC transfer after the liquid staking function
        /// is executed
        /// If None, no IBC transfer will be executed
        /// If set, then `receiver` must be an IBC address
        transfer_channel: Option<String>,
    },
    /// Update the contract configuration
    UpdateConfig {
        /// Flag to enable/disable the contract
        active: Option<bool>,
        /// LS token prefix
        ls_prefix: Option<String>,
        /// IBC timeouts
        timeouts: Option<Timeouts>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LsConfig)]
    LsConfig {},
}

#[cw_serde]
pub struct Timeouts {
    /// ica timeout in seconds
    pub ica_timeout: Uint64,
    /// ibc transfer timeout in seconds
    pub ibc_transfer_timeout: Uint64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            ica_timeout: Uint64::new(DEFAULT_TIMEOUT),
            ibc_transfer_timeout: Uint64::new(DEFAULT_TIMEOUT),
        }
    }
}

#[cw_serde]
pub struct IbcConfig {
    pub ibc_transfer_timeout: Uint64,
    pub ica_timeout: Uint64,
}
