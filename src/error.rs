use cosmwasm_std::{OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not active")]
    NotActive {},

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Payment error: {0}")]
    PaymentError(String),

    #[error("Unknown reply id")]
    UnknownReplyId {},

    #[error("Parse reply error: {0}")]
    ParseReplyError(String),

    #[error("LS failed to return data in its response")]
    LSResponseDataMissing,
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

impl From<PaymentError> for ContractError {
    fn from(e: PaymentError) -> Self {
        ContractError::PaymentError(e.to_string())
    }
}
