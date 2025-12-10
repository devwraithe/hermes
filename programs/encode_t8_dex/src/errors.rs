use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero.")]
    ZeroAmount,
    #[msg("Deposited token amounts do not match the pool ratio.")]
    InvalidRatio,
    #[msg("Insufficient liquidity in the pool.")]
    InsufficientLiquidity,
    #[msg("Insufficient LP tokens to perform this operation.")]
    InsufficientLpTokens,
    #[msg("Calculation overflow occurred.")]
    CalculationOverflow,
    #[msg("Withdrawn token amounts would be zero.")]
    ZeroWithdrawAmount,
    #[msg("Slippage exceeded.")]
    SlippageExceeded,
    #[msg("An error occurred during calculation.")]
    CalculationFailure,
}
