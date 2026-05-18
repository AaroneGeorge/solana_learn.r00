import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BuilderA1 } from "../target/types/builder_a1";
import { PublicKey } from "@solana/web3.js";

describe("builder_a1", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.builderA1 as Program<BuilderA1>;
  const user = anchor.AnchorProvider.env().wallet;

  it("Initialize vault", async () => {
    const [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.publicKey.toBuffer()],
      program.programId
    );

    const tx = await program.methods
      .initialize()
      .accounts({
        user: user.publicKey,
        vaultState: vaultStatePda,
      })
      .rpc();

    console.log("Initialize tx:", tx);
  });

  it("Deposit to vault", async () => {
    const [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.publicKey.toBuffer()],
      program.programId
    );

    const depositAmount = 5_000_000; // 0.005 SOL

    const tx = await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accounts({
        user: user.publicKey,
        vaultState: vaultStatePda,
      })
      .rpc();

    console.log("Deposit tx:", tx);
  });

  it("Withdraw from vault", async () => {
    const [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.publicKey.toBuffer()],
      program.programId
    );

    const [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vaultStatePda.toBuffer()],
      program.programId
    );

    const withdrawAmount = 2_000_000; // 0.002 SOL
    const expectedRemainingBalance = 3_000_000; // 0.005 - 0.002 = 0.003 SOL

    const tx = await program.methods
      .withdraw(new anchor.BN(withdrawAmount))
      .accounts({
        user: user.publicKey,
        vaultState: vaultStatePda,
      })
      .rpc();

    console.log("Withdraw tx:", tx);

    const vaultAccount = await anchor.getProvider().connection.getAccountInfo(vaultPda);
    console.log("Vault balance after withdrawal:", vaultAccount?.lamports, "lamports (0.003 SOL)");
    console.assert(vaultAccount?.lamports === expectedRemainingBalance, `Expected ${expectedRemainingBalance} lamports, got ${vaultAccount?.lamports}`);
  });

  it("Close vault", async () => {
    const [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.publicKey.toBuffer()],
      program.programId
    );

    const [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vaultStatePda.toBuffer()],
      program.programId
    );

    const tx = await program.methods
      .close()
      .accounts({
        user: user.publicKey,
        vaultState: vaultStatePda,
      })
      .rpc();

    console.log("Close tx:", tx);

    const vaultAccount = await anchor.getProvider().connection.getAccountInfo(vaultPda);
    console.log("Vault account after close:", vaultAccount);
    console.assert(vaultAccount === null || vaultAccount.lamports === 0, "Vault account should be closed (0.003 SOL returned to user)");
  });
});
