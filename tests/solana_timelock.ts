import * as anchor from "@coral-xyz/anchor";
const assert = require("assert");

describe("solana_timelock", () => {
  const provider = anchor.getProvider();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaTimelock;

  it("Tests the timelock program", async () => {
    const timelock = anchor.web3.Keypair.generate();
    const [timelockSigner, nonce] =
      await anchor.web3.PublicKey.findProgramAddress(
        [timelock.publicKey.toBuffer()],
        program.programId
      );
    const timelockSize = 200; // Big enough.

    const delayInSlots = new anchor.BN(1);
    const timelockAuthority = anchor.web3.Keypair.generate();

    await program.methods
      .createTimelock(timelockAuthority.publicKey, delayInSlots, nonce)
      .accounts({
        timelock: timelock.publicKey,
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

    const pid = program.programId;
    const accounts = [
      {
        pubkey: timelock.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: timelockSigner,
        isWritable: false,
        isSigner: true,
      },
    ];
    const newDelayInSlots = new anchor.BN(4);
    const data = program.coder.instruction.encode("set_delay_in_slots", {
      delayInSlots: newDelayInSlots,
    });

    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000;
    await program.methods
      .enqueueTransaction(pid, accounts, data)
      .accounts({
        timelock: timelock.publicKey,
        transaction: transaction.publicKey,
        authority: timelockAuthority.publicKey,
      })
      .preInstructions([
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ])
      .signers([transaction, timelockAuthority])
      .rpc();

    const txAccount = await program.account.transaction.fetch(
      transaction.publicKey
    );

    assert.ok(txAccount.programId.equals(pid));
    assert.deepStrictEqual(txAccount.accounts, accounts);
    assert.deepStrictEqual(txAccount.data, data);
    assert.ok(txAccount.timelock.equals(timelock.publicKey));
    assert.deepStrictEqual(txAccount.didExecute, false);

    await program.methods
      .executeTransaction()
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner,
        transaction: transaction.publicKey,
        authority: timelockAuthority.publicKey,
      })
      .remainingAccounts(
        program.instruction.setDelayInSlots
          .accounts({
            timelock: timelock.publicKey,
            timelockSigner,
          })
          // Change the signer status on the vendor signer since it's signed by the program, not the client.
          .map((meta) =>
            meta.pubkey.equals(timelockSigner)
              ? { ...meta, isSigner: false }
              : meta
          )
          .concat({
            pubkey: program.programId,
            isWritable: false,
            isSigner: false,
          })
      )
      .signers([timelockAuthority])
      .rpc()
      .then(
        (m) => assert.fail("Transaction executed even without waiting"),
        (e) => { assert.strictEqual(e.error.errorCode.code, "NotReady") }
      );

    // we do this to move one slot forward
    const dummyTx = new anchor.web3.Transaction();
    dummyTx.add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: provider.publicKey,
        lamports: 10,
      })
    );
    await provider.sendAndConfirm(dummyTx);

    await program.methods
      .executeTransaction()
      .accounts({
        timelock: timelock.publicKey,
        timelockSigner,
        transaction: transaction.publicKey,
        authority: timelockAuthority.publicKey,
      })
      .remainingAccounts(
        program.instruction.setDelayInSlots
          .accounts({
            timelock: timelock.publicKey,
            timelockSigner,
          })
          // Change the signer status on the vendor signer since it's signed by the program, not the client.
          .map((meta) =>
            meta.pubkey.equals(timelockSigner)
              ? { ...meta, isSigner: false }
              : meta
          )
          .concat({
            pubkey: program.programId,
            isWritable: false,
            isSigner: false,
          })
      )
      .signers([timelockAuthority])
      .rpc();

    timelockAccount = await program.account.timelock.fetch(timelock.publicKey);

    assert.strictEqual(timelockAccount.signerBump, nonce);
    assert.ok(timelockAccount.delayInSlots.eq(newDelayInSlots));
  });
});
