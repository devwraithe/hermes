use crate::errors::ErrorCode;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

pub fn handler(ctx: Context<AddLiquidity>, amount_a: u64, amount_b: u64) -> Result<()> {
    if amount_a == 0 || amount_b == 0 {
        return err!(ErrorCode::ZeroAmount);
    }

    let lp_amount_to_mint: u64;

    // Vault balances
    let vault_a_balance = ctx.accounts.token_vault_a.amount;
    let vault_b_balance = ctx.accounts.token_vault_b.amount;
    let lp_mint_supply = ctx.accounts.lp_mint.supply;

    // Check if there is any liquidity in the pool
    if lp_mint_supply == 0 {
        // No liquidity
        // lp_amount_to_mint = sqrt(amount_a * amount_b)
        // (UniswapV2 does this for protection from price manipulation attack)
        lp_amount_to_mint = (amount_a as u128 * amount_b as u128)
            .isqrt()
            .try_into()?;

        if lp_amount_to_mint == 0 {
            return err!(ErrorCode::ZeroAmount);
        }

    } else {
        // There is some liquidity
        // We have to check the proportion is correct
        // required_b = amount_a * vault_b_balance / vault_a_balance
        let required_b = (amount_a as u128 * vault_b_balance as u128) / vault_a_balance as u128;

        if required_b != amount_b as u128 {
            return err!(ErrorCode::InvalidRatio);
        }

        // lp_to_mint = amount_a * lp_mint_supply / vault_a_balance
        lp_amount_to_mint = ((amount_a as u128 * lp_mint_supply as u128) / vault_a_balance as u128)
            .try_into()?;
    }

    // Transfer tokens from user to vaults
    token::transfer(ctx.accounts.transfer_a_context(), amount_a)?;
    token::transfer(ctx.accounts.transfer_b_context(), amount_b)?;

    let seeds = &[
        b"pool",
        ctx.accounts.mint_a.to_account_info().key.as_ref(),
        ctx.accounts.mint_b.to_account_info().key.as_ref(),
        &[ctx.accounts.pool.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.lp_mint.to_account_info(),
        to: ctx.accounts.user_lp_token_account.to_account_info(),
        authority: ctx.accounts.pool.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

    token::mint_to(cpi_ctx, lp_amount_to_mint)?;

    msg!(
        "Liquidity added: {} of token A, {} of token B.",
        amount_a,
        amount_b
    );
    msg!("Minted {} LP tokens.", lp_amount_to_mint);

    Ok(())
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
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

    // Function caller
    #[account(mut)]
    pub user: Signer<'info>,

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

    // User token account for LP tokens
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Functions for CPI context creation
impl<'info> AddLiquidity<'info> {
    pub fn transfer_a_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account_a.to_account_info(),
                to: self.token_vault_a.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }

    pub fn transfer_b_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account_b.to_account_info(),
                to: self.token_vault_b.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}
