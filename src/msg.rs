use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    /// LS token prefix
    pub ls_prefix: String,
}

#[cw_serde]
pub struct LsConfig {
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
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LsConfig)]
    LsConfig {},
}
