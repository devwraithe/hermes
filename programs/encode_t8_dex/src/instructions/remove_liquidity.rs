use crate::errors::ErrorCode;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, Token, TokenAccount},
};

pub fn handler(ctx: Context<RemoveLiquidity>, lp_amount: u64) -> Result<()> {
    if lp_amount == 0 {
        return err!(ErrorCode::ZeroAmount);
    }

    let vault_a_balance = ctx.accounts.token_vault_a.amount;
    let vault_b_balance = ctx.accounts.token_vault_b.amount;
    let lp_mint_supply = ctx.accounts.lp_mint.supply;

    // Check if there is any liquidity in the pool
    if lp_mint_supply == 0 {
        // No liquidity
        return err!(ErrorCode::InsufficientLiquidity);
    } else {
        // There is liquidity, ensure user has enough LP tokens
        if ctx.accounts.user_lp_token_account.amount < lp_amount {
            return err!(ErrorCode::InsufficientLpTokens);
        }

        // Calculate amounts to withdraw
        let amount_a = (lp_amount as u128)
            .checked_mul(vault_a_balance as u128)
            .and_then(|v| v.checked_div(lp_mint_supply as u128))
            .ok_or(ErrorCode::CalculationOverflow)? as u64;

        let amount_b = (lp_amount as u128)
            .checked_mul(vault_b_balance as u128)
            .and_then(|v| v.checked_div(lp_mint_supply as u128))
            .ok_or(ErrorCode::CalculationOverflow)? as u64;

        if amount_a == 0 || amount_b == 0 {
            return err!(ErrorCode::ZeroWithdrawAmount);
        }

        // Burn LP tokens from user
        token::burn(ctx.accounts.burn_lp_context(), lp_amount)?;

        let seeds = &[
            b"pool",
            ctx.accounts.mint_a.to_account_info().key.as_ref(),
            ctx.accounts.mint_b.to_account_info().key.as_ref(),
            &[ctx.accounts.pool.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Transfer tokens from vaults to user
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.token_vault_a.to_account_info(),
                    to: ctx.accounts.user_token_account_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            ),
            amount_a,
        )?;

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.token_vault_b.to_account_info(),
                    to: ctx.accounts.user_token_account_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            ),
            amount_b,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    // check PDA
    #[account(
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    // mint_a and mint_b are required to check pool PDA
    #[account(address = pool.mint_a)]
    pub mint_a: Account<'info, Mint>,
    #[account(address = pool.mint_b)]
    pub mint_b: Account<'info, Mint>,

    #[account(
        mut,
        address = pool.token_vault_a
    )]
    pub token_vault_a: Account<'info, TokenAccount>,
    #[account(
        mut,
        address = pool.token_vault_b
    )]
    pub token_vault_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = pool.lp_mint
    )]
    pub lp_mint: Account<'info, Mint>,

    // User token accounts
    #[account(
        mut,
        token::mint = mint_a // check that this vault is for mint_a token
    )]
    pub user_token_account_a: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint_b
    )]
    pub user_token_account_b: Account<'info, TokenAccount>,

    // Function caller
    #[account(mut)]
    pub user: Signer<'info>,

    // User token account for LP tokens
    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Functions for CPI context creation
impl<'info> RemoveLiquidity<'info> {
    pub fn burn_lp_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.lp_mint.to_account_info(),
                from: self.user_lp_token_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}
