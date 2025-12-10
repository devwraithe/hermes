use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    // Token A public key
    pub mint_a: Pubkey,
    // Token B public key
    pub mint_b: Pubkey,
    // Account holding A tokens
    pub token_vault_a: Pubkey,
    // Account holding B tokens
    pub token_vault_b: Pubkey,
    // LP-token
    pub lp_mint: Pubkey,
    // Bump is used for seeds for PDA generation
    pub bump: u8,
}
