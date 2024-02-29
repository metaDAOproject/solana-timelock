import * as anchor from "@coral-xyz/anchor";
const assert = require("assert");

describe("solana_timelock", () => {
  const provider = anchor.getProvider();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaTimelock;

  let timelock: anchor.web3.Keypair;
  let timelockSignerPubkey: anchor.web3.PublicKey;
  let timelockAuthority: anchor.web3.Keypair;

  it("Creates the timelock program", async () => {
    timelock = anchor.web3.Keypair.generate();
    let nonce: number;
    [timelockSignerPubkey, nonce] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [timelock.publicKey.toBuffer()],
        program.programId
      );
    const timelockSize = 200; // Big enough.

    const delayInSlots = new anchor.BN(1);
    timelockAuthority = anchor.web3.Keypair.generate();

    await program.methods
      .createTimelock(timelockAuthority.publicKey, delayInSlots)
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner: timelockSignerPubkey,
      })
      .preInstructions([
        await program.account.timelock.createInstruction(
          timelock,
          timelockSize
        ),
      ])
      .signers([timelock])
      .rpc();

    let timelockAccount = await program.account.timelock.fetch(
      timelock.publicKey
    );
    assert.strictEqual(timelockAccount.signerBump, nonce);
    assert.ok(timelockAccount.delayInSlots.eq(delayInSlots));
  });

  let transactionBatchAuthority: anchor.web3.Keypair;
  let transactionBatch: anchor.web3.Keypair;

  it("Creates a transaction batch", async () => {
    transactionBatchAuthority = anchor.web3.Keypair.generate();
    transactionBatch = anchor.web3.Keypair.generate();

    const transactionBatchSize = 30000; // 3kb ought to be enough

    await program.methods
      .createTransactionBatch()
      .accounts({
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
        timelock: timelock.publicKey,
        transactionBatch: transactionBatch.publicKey,
      })
      .preInstructions([
        await program.account.transactionBatch.createInstruction(
          transactionBatch,
          transactionBatchSize
        ),
      ])
      .signers([transactionBatchAuthority, transactionBatch])
      .rpc();

    const transactionBatchAccount =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);

    // Assert the transaction batch has been created with the correct status and authority
    assert.ok(
      "created" in transactionBatchAccount.status,
      "The batch status should still be 'Created' after creating transaction batch."
    );
    assert.ok(transactionBatchAccount.timelock.equals(timelock.publicKey));
    assert.ok(
      transactionBatchAccount.transactionBatchAuthority.equals(
        transactionBatchAuthority.publicKey
      )
    );
  });

  let recipient: anchor.web3.Keypair;

  it("Adds three transactions to the transaction batch", async () => {
    // First set up a transfer sol instruction
    recipient = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(timelockSignerPubkey, 200_000_000);

    let transferInstruction = anchor.web3.SystemProgram.transfer({
      fromPubkey: timelockSignerPubkey,
      toPubkey: recipient.publicKey,
      lamports: 100_000_000,
    });

    await program.methods
      .addTransaction(
        transferInstruction.programId,
        transferInstruction.keys.map((key) => ({
          pubkey: key.pubkey,
          isSigner: key.isSigner,
          isWritable: key.isWritable,
        })),
        transferInstruction.data
      )
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
      })
      .signers([transactionBatchAuthority])
      .rpc();

    // Next set the timelock delay
    const newDelayInSlots = new anchor.BN(2);
    let setDelayInSlotsInstruction = program.instruction.setDelayInSlots(
      newDelayInSlots,
      {
        accounts: {
          timelock: timelock.publicKey,
          timelockSigner: timelockSignerPubkey,
        },
      }
    );

    await program.methods
      .addTransaction(
        setDelayInSlotsInstruction.programId,
        setDelayInSlotsInstruction.keys.map((key) => ({
          pubkey: key.pubkey,
          isSigner: key.isSigner,
          isWritable: key.isWritable,
        })),
        setDelayInSlotsInstruction.data
      )
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
      })
      .signers([transactionBatchAuthority])
      .rpc();

    // Next change the authority
    let setAuthorityInstruction = program.instruction.setAuthority(
      recipient.publicKey,
      {
        accounts: {
          timelock: timelock.publicKey,
          timelockSigner: timelockSignerPubkey,
        },
      }
    );

    await program.methods
      .addTransaction(
        setAuthorityInstruction.programId,
        setAuthorityInstruction.keys.map((key) => ({
          pubkey: key.pubkey,
          isSigner: key.isSigner,
          isWritable: key.isWritable,
        })),
        setAuthorityInstruction.data
      )
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
      })
      .signers([transactionBatchAuthority])
      .rpc();

    // Finally, assert that the transaction batch contains the three transactions
    const transactionBatchAccount =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);
    assert.strictEqual(
      transactionBatchAccount.transactions.length,
      3,
      "There should be three transactions in the batch."
    );
    assert.ok(
      "created" in transactionBatchAccount.status,
      "The batch status should still be 'Created' after adding transactions."
    );
  });

  it("Seals the transaction batch", async () => {
    await program.methods
      .sealTransactionBatch()
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
      })
      .signers([transactionBatchAuthority])
      .rpc();

    const sealedTransactionBatch = await program.account.transactionBatch.fetch(
      transactionBatch.publicKey
    );

    // Assert the transaction batch is now sealed
    assert.ok(
      "sealed" in sealedTransactionBatch.status,
      "The batch status should be 'Sealed' after sealing."
    );
  });

  it("Enqueues the transaction batch", async () => {
    // Enqueue the transaction batch
    await program.methods
      .enqueueTransactionBatch()
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        authority: timelockAuthority.publicKey,
        timelock: timelock.publicKey,
      })
      .signers([timelockAuthority])
      .rpc();

    // Fetch the updated transaction batch account to verify its status and enqueued slot
    const enqueuedTransactionBatch =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);

    // Assert the transaction batch is now in TimelockStarted status
    assert.ok(
      "enqueued" in enqueuedTransactionBatch.status,
      "The batch status should be 'Enqueued' after enqueueing."
    );

    // Assert the enqueued slot is set
    assert.ok(
      enqueuedTransactionBatch.enqueuedSlot > 0,
      "The enqueued slot should be set and greater than 0."
    );
  });

  it("Dummy transaction to move to the next slot", async () => {
    const dummyTx = new anchor.web3.Transaction();
    dummyTx.add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: provider.publicKey,
        lamports: 10,
      })
    );
    await provider.sendAndConfirm(dummyTx);
    await provider.sendAndConfirm(dummyTx);

    // Fetch the current slot from the blockchain
    const currentSlot = await provider.connection.getSlot();

    // Fetch the transaction batch to get the enqueued slot and delay
    const transactionBatchAccount =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);
    const enqueuedSlot = transactionBatchAccount.enqueuedSlot.toNumber();
    const timelockAccount = await program.account.timelock.fetch(
      timelock.publicKey
    );
    const delayInSlots = timelockAccount.delayInSlots.toNumber();
    const expectedExecutionSlot = enqueuedSlot + delayInSlots;
    // Check if the current slot is greater than the expected execution slot
    assert.ok(
      currentSlot > expectedExecutionSlot,
      `The current slot (${currentSlot}) should be greater than the expected execution slot (${expectedExecutionSlot}).`
    );
  });

  it("Execute the transfer sol transaction in the batch", async () => {
    // First execution call - This will execute the first transaction in the batch
    await program.methods
      .executeTransactionBatch()
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner: timelockSignerPubkey,
        transactionBatch: transactionBatch.publicKey,
      })
      .remainingAccounts([
        // Add remaining accounts needed for the first transaction here (transfer SOL)
        // This includes the from account (which should be the timelock signer), and the to account
        { pubkey: timelockSignerPubkey, isWritable: true, isSigner: false },
        { pubkey: recipient.publicKey, isWritable: true, isSigner: false },
        {
          pubkey: anchor.web3.SystemProgram.programId,
          isWritable: false,
          isSigner: false,
        },
      ])
      .rpc();

    // Verification step
    const transactionBatchAccount =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);

    // Check if the first transaction did execute
    assert.strictEqual(
      transactionBatchAccount.transactions[0].didExecute,
      true,
      "The transfer SOL transaction should have been executed."
    );
    // Verify recipient's balance
    const recipientBalance = await provider.connection.getBalance(
      recipient.publicKey
    );
    assert.strictEqual(
      recipientBalance,
      100_000_000, // This should match the lamports sent in the transaction
      "The recipient's balance should be increased by the amount of lamports sent."
    );
  });

  it("Executes the set delay in slots transaction in the batch", async () => {
    // Execute the second transaction in the batch (set delay in slots)
    await program.methods
      .executeTransactionBatch()
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner: timelockSignerPubkey,
        transactionBatch: transactionBatch.publicKey,
      })
      .remainingAccounts([
        { pubkey: timelockSignerPubkey, isWritable: false, isSigner: false },
        { pubkey: timelock.publicKey, isWritable: true, isSigner: false },
        { pubkey: program.programId, isWritable: false, isSigner: false },
      ])
      .rpc();

    // Verification step
    const transactionBatchAccount =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);

    // Check if the second transaction did execute
    assert.strictEqual(
      transactionBatchAccount.transactions[1].didExecute,
      true,
      "The set delay in slots transaction should have been executed."
    );

    // Fetch the updated Timelock account to verify the delay has been modified
    const updatedTimelockAccount = await program.account.timelock.fetch(
      timelock.publicKey
    );

    // The new delay in slots set by the transaction
    const newDelayInSlots = new anchor.BN(2); // Ensure this matches the value set in the transaction

    // Verify the timelock delay was updated correctly
    assert.ok(
      updatedTimelockAccount.delayInSlots.eq(newDelayInSlots),
      `The delay in slots should be updated to ${newDelayInSlots.toString()}`
    );
  });

  it("Executes the change authority transaction and verifies the update", async () => {
    // Execute the transaction batch to change the authority
    await program.methods
      .executeTransactionBatch()
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner: timelockSignerPubkey,
        transactionBatch: transactionBatch.publicKey,
      })
      // Assuming the remaining accounts are correctly specified as required for the transaction execution
      .remainingAccounts([
        // Include necessary remaining accounts specific to this transaction
        { pubkey: timelockSignerPubkey, isWritable: false, isSigner: false },
        { pubkey: timelock.publicKey, isWritable: true, isSigner: false },
        { pubkey: program.programId, isWritable: false, isSigner: false },
      ])
      .rpc();

    // Fetch the updated TransactionBatch and Timelock account to verify changes
    const updatedTransactionBatch =
      await program.account.transactionBatch.fetch(transactionBatch.publicKey);
    const updatedTimelockAccount = await program.account.timelock.fetch(
      timelock.publicKey
    );

    // Check if the third transaction did execute
    assert.strictEqual(
      updatedTransactionBatch.transactions[2].didExecute,
      true,
      "The change authority transaction should have been executed."
    );
    // Verify the transaction batch status is 'Executed'
    assert.ok(
      "executed" in updatedTransactionBatch.status,
      "The batch status should be 'Executed' after all transactions are processed."
    );

    // Verify the timelock authority was updated correctly
    assert.ok(
      updatedTimelockAccount.authority.equals(recipient.publicKey),
      "The recipient should now be the authority of the timelock."
    );
  });

  it("Creates, seals, enqueues, and then cancels a transaction batch", async () => {
    // Step 1: Create Transaction Batch
    transactionBatchAuthority = anchor.web3.Keypair.generate();
    transactionBatch = anchor.web3.Keypair.generate();

    const transactionBatchSize = 30000; // Adequate size for the transaction batch

    await program.methods
      .createTransactionBatch()
      .accounts({
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
        timelock: timelock.publicKey,
        transactionBatch: transactionBatch.publicKey,
      })
      .preInstructions([
        await program.account.transactionBatch.createInstruction(
          transactionBatch,
          transactionBatchSize
        ),
      ])
      .signers([transactionBatchAuthority, transactionBatch])
      .rpc();

    // Verify the transaction batch has been created
    let transactionBatchAccount = await program.account.transactionBatch.fetch(
      transactionBatch.publicKey
    );
    assert.ok(
      "created" in transactionBatchAccount.status,
      "The transaction batch should be in 'Created' status after creation."
    );

    // Step 2: Seal the Transaction Batch
    await program.methods
      .sealTransactionBatch()
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        transactionBatchAuthority: transactionBatchAuthority.publicKey,
      })
      .signers([transactionBatchAuthority])
      .rpc();

    // Verify the transaction batch has been sealed
    transactionBatchAccount = await program.account.transactionBatch.fetch(
      transactionBatch.publicKey
    );
    assert.ok(
      "sealed" in transactionBatchAccount.status,
      "The transaction batch should be in 'Sealed' status after sealing."
    );

    // Step 3: Enqueue the Transaction Batch
    // Recipient is the new authority of the timelock
    await program.methods
      .enqueueTransactionBatch()
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        authority: recipient.publicKey,
        timelock: timelock.publicKey,
      })
      .signers([recipient])
      .rpc();

    // Verify the transaction batch has been enqueued
    transactionBatchAccount = await program.account.transactionBatch.fetch(
      transactionBatch.publicKey
    );
    assert.ok(
      "enqueued" in transactionBatchAccount.status,
      "The transaction batch should be in 'Enqueued' status after enqueueing."
    );

    // Step 4: Cancel the Transaction Batch
    await program.methods
      .cancelTransactionBatch()
      .accounts({
        transactionBatch: transactionBatch.publicKey,
        authority: recipient.publicKey,
        timelock: timelock.publicKey,
      })
      .signers([recipient])
      .rpc();

    // Verify the transaction batch has been cancelled
    transactionBatchAccount = await program.account.transactionBatch.fetch(
      transactionBatch.publicKey
    );
    assert.ok(
      "cancelled" in transactionBatchAccount.status,
      "The transaction batch should be in 'Cancelled' status after cancellation."
    );
  });
});
