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
pub struct TransactionQueue {
    pub status: TransactionQueueStatus,
    pub transactions: Vec<Transaction>,
    pub timelock: Pubkey,
    pub enqueued_slot: u64,
    pub transaction_queue_authority: Pubkey
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Transaction {
    pub program_id: Pubkey,
    pub accounts: Vec<TransactionAccount>,
    pub data: Vec<u8>,
    pub did_execute: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum TransactionQueueStatus {
    Created,
    Sealed,
    TimelockStarted,
    Void,
    Executed
}

#[program]
pub mod solana_timelock {
    use super::*;

    pub fn create_timelock(
        ctx: Context<CreateTimelock>,
        authority: Pubkey,
        delay_in_slots: u64,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        timelock.authority = authority;
        timelock.delay_in_slots = delay_in_slots;
        timelock.signer_bump = ctx.bumps.timelock_signer;

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

    pub fn create_transaction_queue(
        ctx: Context<CreateTransactionQueue>,
    ) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;

        tx_queue.timelock = ctx.accounts.timelock.key();
        tx_queue.transaction_queue_authority = ctx.accounts.transaction_queue_authority.key();
        tx_queue.status = TransactionQueueStatus::Created;

        Ok(())
    }

    pub fn add_transaction(
        ctx: Context<UpdateTransactionQueue>,
        program_id: Pubkey,
        accounts: Vec<TransactionAccount>,
        data: Vec<u8>
    ) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;

        msg!("Current transaction queue status: {:?}", tx_queue.status);
        require!(tx_queue.status == TransactionQueueStatus::Created, TimelockError::CannotAddTransactions);

        let this_transaction = Transaction {
            program_id,
            accounts,
            data,
            did_execute: false
        };

        tx_queue.transactions.push(this_transaction);

        Ok(())
    }

    pub fn seal_transaction_queue(
        ctx: Context<UpdateTransactionQueue>
    ) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;

        msg!("Current transaction queue status: {:?}", tx_queue.status);
        require!(tx_queue.status == TransactionQueueStatus::Created, TimelockError::CannotSealTransactionQueue);

        tx_queue.status = TransactionQueueStatus::Sealed;

        Ok(())
    }

    pub fn start_timelock(
        ctx: Context<StartOrVoidTimelock>
    ) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;
        let clock = Clock::get()?;

        msg!("Current transaction queue status: {:?}", tx_queue.status);
        require!(tx_queue.status == TransactionQueueStatus::Sealed, TimelockError::CannotStartTimelock);

        tx_queue.status = TransactionQueueStatus::TimelockStarted;
        tx_queue.enqueued_slot = clock.slot;

        Ok(())
    }

    pub fn void_timelock(
        ctx: Context<StartOrVoidTimelock>
    ) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;

        msg!("Current transaction queue status: {:?}", tx_queue.status);
        require!(tx_queue.status == TransactionQueueStatus::TimelockStarted, TimelockError::CannotVoidTimelock);

        let clock = Clock::get()?;
        let enqueued_slot = tx_queue.enqueued_slot;
        let required_delay = ctx.accounts.timelock.delay_in_slots;
        require!(clock.slot - enqueued_slot < required_delay, TimelockError::CanOnlyVoidDuringTimelockPeriod);

        // A fallback option that allows the timelock authority to prevent the
        // transaction queue from executing by voiding it during the timelock period.
        tx_queue.status = TransactionQueueStatus::Void;

        Ok(())

    }

    pub fn execute_transaction_queue(ctx: Context<ExecuteTransactionQueue>) -> Result<()> {
        let tx_queue = &mut ctx.accounts.transaction_queue;

        msg!("Current transaction queue status: {:?}", tx_queue.status);
        require!(tx_queue.status == TransactionQueueStatus::TimelockStarted, TimelockError::CannotExecuteTransactions);

        let clock = Clock::get()?;
        let enqueued_slot = tx_queue.enqueued_slot;
        let required_delay = ctx.accounts.timelock.delay_in_slots;
        require!(clock.slot - enqueued_slot > required_delay, TimelockError::NotReady);

        if let Some(transaction) = tx_queue.transactions.iter_mut().find(|tx| !tx.did_execute) {
            let mut ix: Instruction = transaction.deref().into();
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
    
            transaction.did_execute = true;
        }

        if tx_queue.transactions.iter().all(|tx| tx.did_execute) {
            tx_queue.status = TransactionQueueStatus::Executed;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTimelock<'info> {
    #[account(
        seeds = [timelock.key().as_ref()],
        bump,
    )]
    timelock_signer: SystemAccount<'info>, 
    #[account(zero, signer)]
    timelock: Box<Account<'info, Timelock>>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(
        seeds = [timelock.key().as_ref()],
        bump = timelock.signer_bump,
    )]
    timelock_signer: Signer<'info>,
    #[account(mut)]
    timelock: Box<Account<'info, Timelock>>,
}

#[derive(Accounts)]
pub struct CreateTransactionQueue<'info> {
    transaction_queue_authority: Signer<'info>,
    timelock: Box<Account<'info, Timelock>>,
    #[account(zero, signer)]
    transaction_queue: Box<Account<'info, TransactionQueue>>
}

#[derive(Accounts)]
pub struct UpdateTransactionQueue<'info> {
    transaction_queue_authority: Signer<'info>,
    #[account(has_one=transaction_queue_authority)]
    transaction_queue: Box<Account<'info, TransactionQueue>>
}

#[derive(Accounts)]
pub struct StartOrVoidTimelock<'info> {
    authority: Signer<'info>,
    #[account(has_one = authority)]
    timelock: Box<Account<'info, Timelock>>,
    transaction_queue: Box<Account<'info, TransactionQueue>>
}

#[derive(Accounts)]
pub struct ExecuteTransactionQueue<'info> {
    #[account(
        seeds = [timelock.key().as_ref()],
        bump = timelock.signer_bump,
    )]
    timelock_signer: SystemAccount<'info>,
    timelock: Box<Account<'info, Timelock>>,
    #[account(mut, has_one = timelock)]
    transaction_queue: Box<Account<'info, TransactionQueue>>
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
    #[msg("Can only add instructions when transaction queue status is `Created`")]
    CannotAddTransactions,
    #[msg("Can only seal the transaction queue when status is `Created`")]
    CannotSealTransactionQueue,
    #[msg("Can only start the timelock running once the status is `Sealed`")]
    CannotStartTimelock,
    #[msg("Can only void the transactions if the status `TimelockStarted`")]
    CannotVoidTimelock,
    #[msg("Can only void the transactions during the timelock period")]
    CanOnlyVoidDuringTimelockPeriod,
    #[msg("Can only execute the transactions if the status is `TimelockStarted`")]
    CannotExecuteTransactions
}
