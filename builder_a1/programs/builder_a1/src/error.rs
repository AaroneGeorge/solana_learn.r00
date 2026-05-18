use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,

    // #[msg("Insufficient balance in vault")]
    // InsufficientBalance,

    // #[msg("Unauthorized: only owner can withdraw")]
    // UnauthorizedWithdrawal,
}
