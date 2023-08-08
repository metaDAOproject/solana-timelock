//! A simple program that allows users, DAOs, and multisigs to delay transaction
//! execution. May be useful in enhancing an application's decentralization
//! and/or security.
//!
//! Based off of [coral-xyz/multisig](https://github.com/coral-xyz/multisig).

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use solana_program::instruction::Instruction;
use std::convert::Into;
use std::borrow::Borrow;

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

        let timelock_key = timelock.key();
        let seeds = &[timelock_key.as_ref()];
        let (_, signer_bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        timelock.signer_bump = signer_bump;

        Ok(())
    }

    pub fn enqueue_transaction(
        ctx: Context<EnqueueTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;
        let transaction = &mut ctx.accounts.transaction;
        let clock = Clock::get()?;

        transaction.enqueued_slot = clock.slot;
        transaction.program_id = pid;
        transaction.accounts = accs;
        transaction.data = data;

        timelock.transaction_queue.push(transaction.key());

        Ok(())
    }

//     pub fn dequeue_transaction(ctx: Context<DequeueTransaction>, index: u64) -> Result<()> {
//         let timelock = &mut ctx.accounts.timelock;

//         let removed_tx = timelock.transaction_queue.remove(index as usize);

//         require!(
//             removed_tx == ctx.accounts.transaction.key(),
//             TimelockError::WrongTransactionAccount
//         );

//         Ok(())
//     }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>, index: u64) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        let removed_tx = timelock.transaction_queue.remove(index as usize);

        require!(
            removed_tx == ctx.accounts.transaction.key(),
            TimelockError::WrongTransactionAccount
        );

        let mut ix: Instruction = (*ctx.accounts.transaction).borrow().into();
        // for acc in ix.accounts.iter_mut() {
        //     if &acc.pubkey == ctx.accounts.timelock_signer.key {
        //         acc.is_signer = true;
        //     }
        // }

        let timelock_key = ctx.accounts.timelock.key();
        let seeds = &[timelock_key.as_ref(), &[ctx.accounts.timelock.signer_bump]];
        let signer = &[&seeds[..]];
        // msg!("{:?}", ctx.remaining_accounts);
        let accounts = ctx.remaining_accounts;
        msg!("{:?}", ix);
        // solana_program::program::invoke_signed(&ix, accounts, signer)?;
        solana_program::program::invoke(&ix, accounts)?;

        // Burn the transaction to ensure one time use.
        // ctx.accounts.transaction.did_execute = true;

        Ok(())
    }

    pub fn update_delay_in_slots(
        ctx: Context<RecursiveAuth>,
        new_delay_in_slots: u64,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        msg!("new delay: {}", new_delay_in_slots);

        timelock.delay_in_slots = new_delay_in_slots;

        Ok(())
    }

    pub fn update_delay(
        ctx: Context<Foo>,
        new_delay_in_slots: u64,
    ) -> Result<()> {
        let timelock = &mut ctx.accounts.timelock;

        msg!("new delay: {}", new_delay_in_slots);

        timelock.delay_in_slots = new_delay_in_slots;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Foo<'info> {
    #[account(mut)]
    timelock: Box<Account<'info, Timelock>>,
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
    timelock: Box<Account<'info, Timelock>>,
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

#[derive(Accounts)]
pub struct DequeueTransaction<'info> {
    #[account(mut, has_one = authority)]
    pub timelock: Account<'info, Timelock>,
    pub authority: Signer<'info>,
    #[account(mut, close = lamport_receiver)]
    pub transaction: Account<'info, Transaction>,
    /// CHECK: https://www.eff.org/cyberspace-independence
    #[account(mut)]
    pub lamport_receiver: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(mut)]
    pub timelock: Box<Account<'info, Timelock>>,
    /// CHECK: https://www.eff.org/cyberspace-independence
    // #[account(
    //     seeds = [timelock.key().as_ref()],
    //     bump = timelock.signer_bump,
    // )]
    // timelock_signer: UncheckedAccount<'info>,
    // pub authority: Signer<'info>,
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    // #[account(mut)]
    // pub lamport_receiver: UncheckedAccount<'info>,
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
    #[msg("Tried to dequeue a transaction from the wrong index / timelock")]
    WrongTransactionAccount,
}
