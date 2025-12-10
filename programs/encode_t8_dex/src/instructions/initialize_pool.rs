use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

pub fn handler(ctx: Context<InitializePool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    pool.token_vault_a = ctx.accounts.token_vault_a.key();
    pool.token_vault_b = ctx.accounts.token_vault_b.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.bump = ctx.bumps.pool;

    msg!(
        "Pool initialized for mints: {} and {}",
        pool.mint_a,
        pool.mint_b
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    // Account containing data about state
    #[account(
        init,
        payer = payer,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,

        // 8 byte       for Anchor
        // 32 byte x 5  for 5 Pubkeys
        // 1 byte       for bump
        space = 8 + (32 * 5) + 1
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = payer,
        seeds = [b"lp_mint", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = pool
    )]
    pub lp_mint: Account<'info, Mint>,

    // Token accounts
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    // Account that holds token A
    #[account(
        init,
        payer = payer,
        token::mint = mint_a,
        token::authority = pool // PDA
    )]
    pub token_vault_a: Account<'info, TokenAccount>,
    // Account that holds token B
    #[account(
        init,
        payer = payer,
        token::mint = mint_b,
        token::authority = pool
    )]
    pub token_vault_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
