import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BuilderA2 } from "../target/types/builder_a2";
import { createMint, createAccount, mintTo } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import * as assert from "assert";

describe("builder_a2 - Escrow Program", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.builderA2 as Program<BuilderA2>;
  const provider = anchor.getProvider();
  const connection = provider.connection;

  // Test accounts
  let creator: anchor.web3.Keypair;
  let acceptor: anchor.web3.Keypair;
  let mintA: PublicKey;
  let mintB: PublicKey;
  let creatorAtaA: PublicKey;
  let acceptorAtaB: PublicKey;

  // Escrow parameters
  const seed = new anchor.BN(1);
  const depositAmount = new anchor.BN(1000);
  const receiveAmount = new anchor.BN(1000); // Same as deposit for 1:1 swap

  before(async () => {
    // Create keypairs for creator and acceptor
    creator = anchor.web3.Keypair.generate();
    acceptor = anchor.web3.Keypair.generate();

    // Airdrop SOL to both accounts
    const airdropCreator = await connection.requestAirdrop(
      creator.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    const airdropAcceptor = await connection.requestAirdrop(
      acceptor.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropCreator);
    await connection.confirmTransaction(airdropAcceptor);

    // Create token mints (token A and token B)
    const wallet = provider.wallet as any;
    mintA = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6
    );

    mintB = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6
    );

    // Create ATAs for creator and acceptor
    creatorAtaA = await createAccount(
      connection,
      wallet.payer,
      mintA,
      creator.publicKey
    );

    acceptorAtaB = await createAccount(
      connection,
      wallet.payer,
      mintB,
      acceptor.publicKey
    );

    // Mint tokens to creator (Token A) and acceptor (Token B)
    await mintTo(connection, wallet.payer, mintA, creatorAtaA, wallet.payer, 5000);
    await mintTo(connection, wallet.payer, mintB, acceptorAtaB, wallet.payer, 5000);
  });

  it("Should create an escrow", async () => {
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        creator.publicKey.toBuffer(),
        seed.toBuffer("le", 8),
      ],
      program.programId
    );

    // Find vault ATA
    const vault = anchor.utils.token.associatedAddress({
      mint: mintA,
      owner: escrowPda,
    });

    const tx = await program.methods
      .createEscrow(seed, receiveAmount, depositAmount)
      .accounts({
        creator: creator.publicKey,
        mintA,
        mintB,
        creatorAtaA,
        escrow: escrowPda,
        vault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    console.log("create escrow transaction:", tx);

    // Verify escrow account was created with correct data
    const escrowAccount = await program.account.escrow.fetch(escrowPda);
    console.log("   escrow account:", escrowAccount);

    assert.ok(escrowAccount.seed.eq(seed), "Seed mismatch");
    assert.ok(
      escrowAccount.creator.equals(creator.publicKey),
      "Creator mismatch"
    );
    assert.ok(escrowAccount.mintA.equals(mintA), "Mint A mismatch");
    assert.ok(escrowAccount.mintB.equals(mintB), "Mint B mismatch");
    assert.ok(
      escrowAccount.receive.eq(receiveAmount),
      "Receive amount mismatch"
    );

    // Verify vault has correct amount of tokens
    const vaultAccount = await connection.getTokenAccountBalance(vault);
    console.log("   vault balance:", vaultAccount.value.amount);
    assert.strictEqual(
      vaultAccount.value.amount,
      depositAmount.toString(),
      "Vault balance mismatch"
    );
  });

  it("Should accept an escrow", async () => {
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        creator.publicKey.toBuffer(),
        seed.toBuffer("le", 8),
      ],
      program.programId
    );

    const vault = anchor.utils.token.associatedAddress({
      mint: mintA,
      owner: escrowPda,
    });

    // Create acceptor ATA for token B (to receive from creator)
    const wallet = provider.wallet as any;
    const creatorAtaB = await createAccount(
      connection,
      wallet.payer,
      mintB,
      creator.publicKey
    );

    // Create acceptor ATA for token A (to receive from vault)
    const acceptorAtaA = await createAccount(
      connection,
      wallet.payer,
      mintA,
      acceptor.publicKey
    );

    const tx = await program.methods
      .acceptEscrow()
      .accounts({
        acceptor: acceptor.publicKey,
        creator: creator.publicKey,
        mintA,
        mintB,
        acceptorAtaA,
        acceptorAtaB,
        creatorAtaB,
        escrow: escrowPda,
        vault,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([acceptor])
      .rpc();

    console.log("accept escrow transaction:", tx);

    // Verify acceptor received token A
    const acceptorAtaABalance = await connection.getTokenAccountBalance(acceptorAtaA);
    console.log("   acceptor token a balance:", acceptorAtaABalance.value.amount);
    assert.ok(
      new anchor.BN(acceptorAtaABalance.value.amount).gte(receiveAmount),
      "Acceptor did not receive tokens"
    );

    // Verify creator received token B
    const creatorAtaBBalance = await connection.getTokenAccountBalance(creatorAtaB);
    console.log("   creator token b balance:", creatorAtaBBalance.value.amount);
    assert.strictEqual(
      creatorAtaBBalance.value.amount,
      receiveAmount.toString(),
      "Creator did not receive correct token B amount"
    );

    // Verify escrow and vault are closed
    const escrowExists = await connection.getAccountInfo(escrowPda);
    const vaultExists = await connection.getAccountInfo(vault);
    assert.ok(
      escrowExists === null || escrowExists.lamports === 0,
      "Escrow account should be closed"
    );
    assert.ok(
      vaultExists === null || vaultExists.lamports === 0,
      "Vault account should be closed"
    );
  });

  it("Should refund an escrow", async () => {
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        creator.publicKey.toBuffer(),
        seed.toBuffer("le", 8),
      ],
      program.programId
    );

    // Create a new escrow for refund test
    const seed2 = new anchor.BN(2);
    const [escrowPda2] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        creator.publicKey.toBuffer(),
        seed2.toBuffer("le", 8),
      ],
      program.programId
    );

    const vault2 = anchor.utils.token.associatedAddress({
      mint: mintA,
      owner: escrowPda2,
    });

    // Create the escrow first
    const createTx = await program.methods
      .createEscrow(seed2, receiveAmount, depositAmount)
      .accounts({
        creator: creator.publicKey,
        mintA,
        mintB,
        creatorAtaA,
        escrow: escrowPda2,
        vault: vault2,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    console.log("created escrow for refund test:", createTx);

    // Now refund it
    const refundTx = await program.methods
      .refundEscrow()
      .accounts({
        creator: creator.publicKey,
        mintA,
        creatorAtaA,
        escrow: escrowPda2,
        vault: vault2,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    console.log("refund escrow transaction:", refundTx);

    // Verify creator got tokens back
    const creatorAtaABalance = await connection.getTokenAccountBalance(creatorAtaA);
    console.log("creator token a balance after refund:", creatorAtaABalance.value.amount);
    assert.ok(
      new anchor.BN(creatorAtaABalance.value.amount).gt(new anchor.BN(0)),
      "Creator should have tokens after refund"
    );

    // Verify escrow and vault are closed
    const escrowExists = await connection.getAccountInfo(escrowPda2);
    const vaultExists = await connection.getAccountInfo(vault2);
    assert.ok(
      escrowExists === null || escrowExists.lamports === 0,
      "Escrow account should be closed"
    );
    assert.ok(
      vaultExists === null || vaultExists.lamports === 0,
      "Vault account should be closed"
    );
  });
});
