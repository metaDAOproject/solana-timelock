use anchor_lang::prelude::*;

declare_id!("7wTNNa26MRFt18kKPz6t3oD3RuKcfN3PjUjLG9tHbWH2");

#[program]
pub mod solana_timelock {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
