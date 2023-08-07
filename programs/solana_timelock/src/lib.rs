//! A simple program that allows users, DAOs, and multisigs to delay transaction
//! execution. May be useful in enhancing an application's decentralization
//! and/or security.

use anchor_lang::prelude::*;

declare_id!("7wTNNa26MRFt18kKPz6t3oD3RuKcfN3PjUjLG9tHbWH2");

#[account]
pub struct Timelock {
    pub authority: Pubkey,
    pub delay_in_slots: u64,
    /// PDA bump used to derive the signer of the timelock. The timelock itself
    /// cannot sign because it is not a PDA.
    pub signer_bump: u8,
    // TODO: use a more optimized structure like a sokoban::Deque
    pub transaction_queue: Vec<Pubkey>,
}

#[account]
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

    pub fn init_timelock(
        ctx: Context<InitializeTimelock>,
        authority: Pubkey,
        delay_in_slots: u64,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.authority = authority;
        timelock.delay_in_slots = delay_in_slots;

        Ok(())
    }

    pub fn enqueue_transaction(
        ctx: Context<EnqueueTransaction>,
        program_id: Pubkey,
        accounts: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;
        let transaction = &mut ctx.accounts.transaction;
        let clock = Clock::get()?;

        transaction.enqueued_slot = clock.slot;
        transaction.program_id = program_id;
        transaction.accounts = accounts;
        transaction.data = data;

        timelock.transaction_queue.push(transaction.key());

        Ok(())
    }

    pub fn update_delay_in_slots(
        ctx: Context<RecursiveAuth>,
        new_delay_in_slots: u64,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.delay_in_slots = new_delay_in_slots;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeTimelock<'info> {
    #[account(zero)]
    timelock: Account<'info, Timelock>,
}

/// Instructions with this context need to be executed by the timelock
#[derive(Accounts)]
pub struct RecursiveAuth<'info> {
    #[account(mut)]
    timelock: Account<'info, Timelock>,
    #[account(
        seeds = [timelock.key().as_ref()],
        bump = timelock.signer_bump,
    )]
    timelock_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct EnqueueTransaction<'info> {
    #[account(mut, has_one = authority)]
    pub timelock: Account<'info, Timelock>,
    pub authority: Signer<'info>,
    #[account(zero)]
    pub transaction: Account<'info, Transaction>,
}
