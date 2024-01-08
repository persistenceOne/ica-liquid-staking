use cosmwasm_std::{OverflowError, StdError, CheckedFromRatioError};
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

    #[error("Divide ratio error: {0}")]
    CheckedDivideRatioError(String),
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

impl From<CheckedFromRatioError> for ContractError {
    fn from(e: CheckedFromRatioError) -> Self {
        ContractError::CheckedDivideRatioError(e.to_string())
    }
}
