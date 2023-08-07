import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaTimelock } from "../target/types/solana_timelock";

describe("solana_timelock", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SolanaTimelock as Program<SolanaTimelock>;

  it("Is initialized!", async () => {
    // Add your test here.
    const timelock = anchor.web3.Keypair.generate();
    const [timelockSigner, signerBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [timelock.publicKey.toBuffer()],
        program.programId
      );
    // allow 128 queued transactions at a time
    const timelockCapacity = 128;

    const authority = anchor.web3.Keypair.generate();
    // 1 day delay, * 5 and / 2 is because there are 5 slots every 2 seconds
    const delayInSlots = new anchor.BN((60 * 60 * 24 * 5) / 2);

    const tx = await program.methods
      .initTimelock(authority.publicKey, delayInSlots)
      .accounts({
        timelock: timelock.publicKey,
      })
      .preInstructions([
        await program.account.timelock.createInstruction(
          timelock,
          32 + 8 + 4 + timelockCapacity
        ),
      ])
      .signers([timelock])
      .rpc();

    console.log(await program.account.timelock.fetch(timelock.publicKey));

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
    const data = program.coder.instruction.encode("update_delay_in_slots", {
      new_delay_in_slots: delayInSlots.muln(2),
    });

    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000;

    await program.methods
      .enqueueTransaction(program.programId, accounts, data)
      .accounts({
        timelock: timelock.publicKey,
        authority: authority.publicKey,
        transaction: transaction.publicKey,
      })
      .preInstructions([
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ])
      .signers([authority, transaction])
      .rpc();

    console.log(await program.account.transaction.fetch(transaction.publicKey));
  });
});
