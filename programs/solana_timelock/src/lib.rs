//! A simple program that allows users, DAOs, and multisigs to delay transaction
//! execution. May be useful in enhancing an application's decentralization 
//! and/or security.

use anchor_lang::prelude::*;

declare_id!("7wTNNa26MRFt18kKPz6t3oD3RuKcfN3PjUjLG9tHbWH2");

#[account]
pub struct Timelock {
    pub authority: Pubkey,
    pub delay_in_slots: u64,
    // TODO: use a more optimized structure like a sokoban::Deque
    pub transaction_queue: Vec<Pubkey>, 
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Transaction {
    /// Slot that this transaction was enqueued
    pub enqueued_slot: u64,
    /// Target program to execute against
    pub program_id: Pubkey,
    /// Accounts required for the transaction
    pub accounts: Vec<TransactionAccount>,
    /// Instruction data for the transaction
    pub data: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[program]
pub mod solana_timelock {
    use super::*;

    pub fn init_timelock(ctx: Context<InitializeTimelock>, authority: Pubkey, delay_in_slots: u64) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.authority = authority;
        timelock.delay_in_slots = delay_in_slots;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeTimelock<'info> {
    #[account(zero)]
    timelock: Account<'info, Timelock>,
}

