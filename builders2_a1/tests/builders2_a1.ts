import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Builders2A1 } from "../target/types/builders2_a1";
import { PublicKey, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, mintTo, createAssociatedTokenAccount } from "@solana/spl-token";
import { expect } from "chai";

describe("builders2_a1", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.builders2A1 as Program<Builders2A1>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  let mintX: PublicKey;
  let mintY: PublicKey;
  let user: Keypair;
  let userTokenXAccount: PublicKey;
  let userTokenYAccount: PublicKey;

  const seed = new BN(1);
  const fee = 30; // 0.3% fee in basis points

  before(async () => {
    user = Keypair.generate();

    const airdropSignature = await provider.connection.requestAirdrop(
      user.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSignature);

    mintX = await createMint(
      provider.connection,
      user,
      user.publicKey,
      null,
      6
    );

    mintY = await createMint(
      provider.connection,
      user,
      user.publicKey,
      null,
      6
    );

    userTokenXAccount = await createAssociatedTokenAccount(
      provider.connection,
      user,
      mintX,
      user.publicKey
    );

    userTokenYAccount = await createAssociatedTokenAccount(
      provider.connection,
      user,
      mintY,
      user.publicKey
    );

    await mintTo(
      provider.connection,
      user,
      mintX,
      userTokenXAccount,
      user.publicKey,
      1_000_000 * 10 ** 6
    );

    await mintTo(
      provider.connection,
      user,
      mintY,
      userTokenYAccount,
      user.publicKey,
      1_000_000 * 10 ** 6
    );

    console.log("Setup complete:");
    console.log("  User:", user.publicKey.toBase58());
    console.log("  Mint X:", mintX.toBase58());
    console.log("  Mint Y:", mintY.toBase58());
  });

  it("Initialize pool", async () => {
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    );

    const vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });

    const vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    const tx = await program.methods
      .initialize(seed, fee, null)
      .accounts({
        initializer: provider.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        config,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Initialize transaction:", tx);

    const configAccount = await program.account.config.fetch(config);
    console.log("Config created:", {
      seed: configAccount.seed.toString(),
      fee: configAccount.fee,
      mintX: configAccount.mintX.toBase58(),
      mintY: configAccount.mintY.toBase58(),
      locked: configAccount.locked,
    });

    expect(configAccount.fee).to.equal(fee, "Fee should match");
    expect(configAccount.mintX.toBase58()).to.equal(
      mintX.toBase58(),
      "Mint X should match"
    );
    expect(configAccount.mintY.toBase58()).to.equal(
      mintY.toBase58(),
      "Mint Y should match"
    );
  });

  it("Deposit liquidity", async () => {
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    );

    const vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });

    const vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    const userLpAccount = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: user.publicKey,
    });

    const lpAmount = new BN(1000 * 10 ** 6);
    const maxX = new BN(100_000 * 10 ** 6);
    const maxY = new BN(50_000 * 10 ** 6);

    const tx = await program.methods
      .deposit(lpAmount, maxX, maxY)
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userTokenXAccount,
        userY: userTokenYAccount,
        userLp: userLpAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Deposit transaction:", tx);

    const vaultXAccount = await provider.connection.getTokenAccountBalance(vaultX);
    const vaultYAccount = await provider.connection.getTokenAccountBalance(vaultY);

    console.log("Vault balances after deposit:", {
      vaultX: vaultXAccount.value.amount,
      vaultY: vaultYAccount.value.amount,
    });

    expect(parseInt(vaultXAccount.value.amount)).to.be.greaterThan(0, "Vault X should have tokens");
    expect(parseInt(vaultYAccount.value.amount)).to.be.greaterThan(0, "Vault Y should have tokens");
  });

  it("Swap tokens", async () => {
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    );

    const vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });

    const vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    const initialUserXBalance = await provider.connection.getTokenAccountBalance(
      userTokenXAccount
    );
    const initialVaultYBalance = await provider.connection.getTokenAccountBalance(vaultY);

    console.log("Before swap:", {
      userXBalance: initialUserXBalance.value.amount,
      vaultYBalance: initialVaultYBalance.value.amount,
    });

    const amountIn = new BN(10_000 * 10 ** 6);
    const minAmountOut = new BN(4_000 * 10 ** 6);

    const tx = await program.methods
      .swap(true, amountIn, minAmountOut)
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userTokenXAccount,
        userY: userTokenYAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Swap transaction:", tx);

    const finalUserXBalance = await provider.connection.getTokenAccountBalance(
      userTokenXAccount
    );
    const finalUserYBalance = await provider.connection.getTokenAccountBalance(
      userTokenYAccount
    );
    const finalVaultYBalance = await provider.connection.getTokenAccountBalance(vaultY);

    console.log("After swap:", {
      userXBalance: finalUserXBalance.value.amount,
      userYBalance: finalUserYBalance.value.amount,
      vaultYBalance: finalVaultYBalance.value.amount,
    });

    expect(parseInt(finalUserXBalance.value.amount)).to.be.lessThan(
      parseInt(initialUserXBalance.value.amount),
      "User should have less token X after selling"
    );
    expect(parseInt(finalUserYBalance.value.amount)).to.be.greaterThan(0, "User should receive token Y");
    expect(parseInt(finalUserYBalance.value.amount)).to.be.greaterThanOrEqual(
      minAmountOut.toNumber(),
      "User should receive at least min amount out"
    );
  });

  it("Withdraw liquidity", async () => {
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    );

    const vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });

    const vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    const userLpAccount = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: user.publicKey,
    });

    const lpBalance = await provider.connection.getTokenAccountBalance(userLpAccount);
    console.log("User LP balance:", lpBalance.value.amount);

    const initialUserXBalance = await provider.connection.getTokenAccountBalance(
      userTokenXAccount
    );
    const initialUserYBalance = await provider.connection.getTokenAccountBalance(
      userTokenYAccount
    );

    console.log("Before withdraw:", {
      userXBalance: initialUserXBalance.value.amount,
      userYBalance: initialUserYBalance.value.amount,
      lpBalance: lpBalance.value.amount,
    });

    const lpAmount = new BN(500 * 10 ** 6);
    const minX = new BN(50_000 * 10 ** 6);
    const minY = new BN(20_000 * 10 ** 6);

    const tx = await program.methods
      .withdraw(lpAmount, minX, minY)
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userTokenXAccount,
        userY: userTokenYAccount,
        userLp: userLpAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Withdraw transaction:", tx);

    const finalLpBalance = await provider.connection.getTokenAccountBalance(userLpAccount);
    const finalUserXBalance = await provider.connection.getTokenAccountBalance(
      userTokenXAccount
    );
    const finalUserYBalance = await provider.connection.getTokenAccountBalance(
      userTokenYAccount
    );

    console.log("After withdraw:", {
      userXBalance: finalUserXBalance.value.amount,
      userYBalance: finalUserYBalance.value.amount,
      lpBalance: finalLpBalance.value.amount,
    });

    expect(parseInt(finalLpBalance.value.amount)).to.be.lessThan(
      parseInt(lpBalance.value.amount),
      "User should have less LP tokens after withdrawal"
    );
    expect(parseInt(finalUserXBalance.value.amount)).to.be.greaterThanOrEqual(
      parseInt(initialUserXBalance.value.amount),
      "User should receive token X"
    );
    expect(parseInt(finalUserYBalance.value.amount)).to.be.greaterThanOrEqual(
      parseInt(initialUserYBalance.value.amount),
      "User should receive token Y"
    );
  });

  it("Invalid amount should fail", async () => {
    const [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId
    );

    const vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });

    const vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    const lpAmount = new BN(0);
    const maxX = new BN(100_000 * 10 ** 6);
    const maxY = new BN(50_000 * 10 ** 6);

    const userLpAccount = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: user.publicKey,
    });

    try {
      await program.methods
        .deposit(lpAmount, maxX, maxY)
        .accounts({
          user: user.publicKey,
          mintX,
          mintY,
          config,
          mintLp,
          vaultX,
          vaultY,
          userX: userTokenXAccount,
          userY: userTokenYAccount,
          userLp: userLpAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      throw new Error("Should have thrown error for invalid amount");
    } catch (error) {
      console.log("Correctly caught error for invalid amount:", error.message);
      expect(
        error.message.includes("InvalidAmount") || error.message.includes("Invalid amount")
      ).to.be.true;
    }
  });
});
