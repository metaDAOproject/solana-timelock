//! A simple program that allows users, DAOs, and multisigs to delay transaction
//! execution. May be useful in enhancing an application's decentralization
//! and/or security.

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use std::convert::Into;
use std::ops::Deref;

declare_id!("TiMEYuk7rCBAFYMvhN3hae9PRc1NUYL71Zu3MCaCBVe");

#[account]
pub struct Timelock {
    pub authority: Pubkey,
    pub signer_bump: u8,
    pub delay_in_slots: u64,
}

#[account]
pub struct Transaction {
    pub timelock: Pubkey,
    pub program_id: Pubkey,
    pub accounts: Vec<TransactionAccount>,
    pub data: Vec<u8>,
    pub did_execute: bool,
    pub enqueued_slot: u64,
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

    pub fn create_timelock(
        ctx: Context<CreateTimelock>,
        authority: Pubkey,
        delay_in_slots: u64,
        signer_bump: u8,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.authority = authority;
        timelock.delay_in_slots = delay_in_slots;
        timelock.signer_bump = signer_bump;

        Ok(())
    }

    pub fn enqueue_transaction(
        ctx: Context<EnqueueTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        let tx = &mut ctx.accounts.transaction;
        let clock = Clock::get()?;

        tx.enqueued_slot = clock.slot;
        tx.program_id = pid;
        tx.accounts = accs;
        tx.data = data;
        tx.timelock = ctx.accounts.timelock.key();
        tx.did_execute = false;

        Ok(())
    }

    pub fn set_delay_in_slots(ctx: Context<Auth>, delay_in_slots: u64) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.delay_in_slots = delay_in_slots;

        Ok(())
    }

    pub fn set_authority(ctx: Context<Auth>, authority: Pubkey) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.authority = authority;

        Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        if ctx.accounts.transaction.did_execute {
            return Err(TimelockError::AlreadyExecuted.into());
        }

        let clock = Clock::get()?;
        let enqueued_slot = ctx.accounts.transaction.enqueued_slot;
        let required_delay = ctx.accounts.timelock.delay_in_slots;
        if clock.slot - enqueued_slot < required_delay {
            return Err(TimelockError::NotReady.into());
        }

        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        for acc in ix.accounts.iter_mut() {
            if &acc.pubkey == ctx.accounts.timelock_signer.key {
                acc.is_signer = true;
            }
        }
        let timelock_key = ctx.accounts.timelock.key();
        let seeds = &[timelock_key.as_ref(), &[ctx.accounts.timelock.signer_bump]];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, accounts, signer)?;

        ctx.accounts.transaction.did_execute = true;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTimelock<'info> {
    #[account(zero, signer)]
    timelock: Box<Account<'info, Timelock>>,
}

#[derive(Accounts)]
pub struct EnqueueTransaction<'info> {
    #[account(has_one = authority)]
    timelock: Box<Account<'info, Timelock>>,
    #[account(zero, signer)]
    transaction: Box<Account<'info, Transaction>>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    timelock: Box<Account<'info, Timelock>>,
    #[account(
        seeds = [timelock.key().as_ref()],
        bump = timelock.signer_bump,
    )]
    timelock_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(has_one = authority)]
    timelock: Box<Account<'info, Timelock>>,
    /// CHECK: timelock_signer is a PDA program signer. Data is never read or written to
    #[account(
        seeds = [timelock.key().as_ref()],
        bump = timelock.signer_bump,
    )]
    timelock_signer: UncheckedAccount<'info>,
    #[account(mut, has_one = timelock)]
    transaction: Box<Account<'info, Transaction>>,
    authority: Signer<'info>,
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}


impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[error_code]
pub enum TimelockError {
    #[msg("The given transaction has already been executed")]
    AlreadyExecuted,
    #[msg("This transaction is not yet ready to be executed")]
    NotReady,
}
