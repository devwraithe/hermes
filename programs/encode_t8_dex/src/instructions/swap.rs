use crate::errors::ErrorCode;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Fee = 0.1%
const FEE_RATE: u128 = 1;
const FEE_DENOMINATOR: u128 = 1000;

pub fn handler(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
    if amount_in == 0 {
        return err!(ErrorCode::ZeroAmount);
    }

    let (vault_in, vault_out) =
        if ctx.accounts.token_vault_a.mint == ctx.accounts.user_token_account_in.mint {
            (&ctx.accounts.token_vault_a, &ctx.accounts.token_vault_b)
        } else {
            (&ctx.accounts.token_vault_b, &ctx.accounts.token_vault_a)
        };

    let fee = (amount_in as u128)
        .checked_mul(FEE_RATE)
        .and_then(|result| result.checked_div(FEE_DENOMINATOR))
        .ok_or(ErrorCode::CalculationFailure)?;

    let amount_in_after_fee = (amount_in as u128)
        .checked_sub(fee)
        .ok_or(ErrorCode::CalculationFailure)?;

    let vault_in_balance = vault_in.amount as u128;
    let vault_out_balance = vault_out.amount as u128;

    // amount_out = (vault_out_balance * amount_in_after_fee) / (vault_in_balance + amount_in_after_fee)
    let amount_out = vault_out_balance
        .checked_mul(amount_in_after_fee)
        .and_then(|numerator| {
            vault_in_balance
                .checked_add(amount_in_after_fee)
                .and_then(|denominator| numerator.checked_div(denominator))
        })
        .ok_or(ErrorCode::CalculationFailure)?;

    let amount_out_u64: u64 = amount_out.try_into()?;

    if amount_out_u64 < min_amount_out {
        return err!(ErrorCode::SlippageExceeded);
    }

    // transfer from user to vault
    let transfer_to_vault_cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account_in.to_account_info(),
        to: vault_in.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let transfer_to_vault_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_to_vault_cpi_accounts,
    );
    token::transfer(transfer_to_vault_cpi_ctx, amount_in)?;

    let seeds = &[
        b"pool",
        ctx.accounts.pool.mint_a.as_ref(),
        ctx.accounts.pool.mint_b.as_ref(),
        &[ctx.accounts.pool.bump],
    ];
    let signer = &[&seeds[..]];

    // transfer from vault to user
    let transfer_from_vault_cpi_accounts = Transfer {
        from: vault_out.to_account_info(),
        to: ctx.accounts.user_token_account_out.to_account_info(),
        authority: ctx.accounts.pool.to_account_info(),
    };
    let transfer_from_vault_cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_from_vault_cpi_accounts,
        signer,
    );
    token::transfer(transfer_from_vault_cpi_ctx, amount_out_u64)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(
        // Используем mint_a и mint_b из структуры для большей безопасности
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(address = pool.mint_a)]
    pub mint_a: Account<'info, Mint>,
    #[account(address = pool.mint_b)]
    pub mint_b: Account<'info, Mint>,

    #[account(mut, address = pool.token_vault_a)]
    pub token_vault_a: Account<'info, TokenAccount>,

    #[account(mut, address = pool.token_vault_b)]
    pub token_vault_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account_in.mint == mint_a.key() || user_token_account_in.mint == mint_b.key()
    )]
    pub user_token_account_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_account_out.mint == mint_a.key() || user_token_account_out.mint == mint_b.key(),
        constraint = user_token_account_in.mint != user_token_account_out.mint
    )]
    pub user_token_account_out: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}
