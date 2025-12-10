import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { EncodeT8Dex } from "../target/types/encode_t8_dex";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { assert } from "chai";

// Format token amounts for readability
function formatTokens(amount: number | bigint, decimals: number = 6): string {
  return (Number(amount) / Math.pow(10, decimals)).toFixed(2);
}

// Generate Solana Explorer link
function explorerLink(signature: string): string {
  const isLocal =
    !process.env.ANCHOR_PROVIDER_URL?.includes("devnet") &&
    !process.env.ANCHOR_PROVIDER_URL?.includes("mainnet");

  if (isLocal) {
    return `https://explorer.solana.com/tx/${signature}?cluster=custom&customUrl=http://localhost:8899`;
  }
  const cluster = process.env.ANCHOR_PROVIDER_URL?.includes("devnet")
    ? "devnet"
    : "mainnet-beta";
  return `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`;
}

describe("encode_t8_dex", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EncodeT8Dex as Program<EncodeT8Dex>;
  const payer = provider.wallet as anchor.Wallet;

  const mintA_KP = anchor.web3.Keypair.generate();
  const mintB_KP = anchor.web3.Keypair.generate();
  const mintA = mintA_KP.publicKey;
  const mintB = mintB_KP.publicKey;

  let userTokenAccountA: anchor.web3.PublicKey;
  let userTokenAccountB: anchor.web3.PublicKey;
  let poolPda: anchor.web3.PublicKey;

  console.log("\n=== Test Configuration ===");
  console.log("Program:", program.programId.toString());
  console.log("Mint A:", mintA.toString());
  console.log("Mint B:", mintB.toString());

  it("Initializes a new liquidity pool", async () => {
    console.log("\n--- Test 1: Initialize Pool ---");

    // Create mints
    await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6,
      mintA_KP
    );
    await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6,
      mintB_KP
    );

    [poolPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), mintA.toBuffer(), mintB.toBuffer()],
      program.programId
    );
    const [lpMintPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), mintA.toBuffer(), mintB.toBuffer()],
      program.programId
    );

    console.log("Pool PDA:", poolPda.toString());

    const tokenVaultA_KP = anchor.web3.Keypair.generate();
    const tokenVaultB_KP = anchor.web3.Keypair.generate();

    const sig = await program.methods
      .initializePool()
      .accountsStrict({
        pool: poolPda,
        mintA: mintA,
        mintB: mintB,
        lpMint: lpMintPda,
        tokenVaultA: tokenVaultA_KP.publicKey,
        tokenVaultB: tokenVaultB_KP.publicKey,
        payer: payer.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([tokenVaultA_KP, tokenVaultB_KP])
      .rpc();

    console.log("Transaction:", sig);
    console.log("Explorer:", explorerLink(sig));

    const poolAccount = await program.account.pool.fetch(poolPda);
    assert.ok(poolAccount, "Pool account should exist after initialization.");
  });

  it("Adds the initial liquidity to the pool", async () => {
    console.log("\n--- Test 2: Add Initial Liquidity ---");

    // Setup user token accounts
    userTokenAccountA = await createAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mintA,
      payer.publicKey
    );
    userTokenAccountB = await createAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mintB,
      payer.publicKey
    );

    // Mint tokens to user
    await mintTo(
      provider.connection,
      payer.payer,
      mintA,
      userTokenAccountA,
      payer.payer,
      200_000_000
    );
    await mintTo(
      provider.connection,
      payer.payer,
      mintB,
      userTokenAccountB,
      payer.payer,
      200_000_000
    );

    const amountA = new anchor.BN(100_000_000); // 100 tokens
    const amountB = new anchor.BN(100_000_000); // 100 tokens

    console.log("Depositing:", formatTokens(amountA.toNumber()), "Token A");
    console.log("Depositing:", formatTokens(amountB.toNumber()), "Token B");

    const poolAccount = await program.account.pool.fetch(poolPda);
    const userLpTokenAccount = await getAssociatedTokenAddress(
      poolAccount.lpMint,
      payer.publicKey
    );

    // Balances before
    const userA_before = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_before = await getAccount(
      provider.connection,
      userTokenAccountB
    );

    console.log("\nBefore:");
    console.log("  User Token A:", formatTokens(userA_before.amount));
    console.log("  User Token B:", formatTokens(userB_before.amount));

    const sig = await program.methods
      .addLiquidity(amountA, amountB)
      .accountsStrict({
        pool: poolPda,
        mintA: poolAccount.mintA,
        mintB: poolAccount.mintB,
        tokenVaultA: poolAccount.tokenVaultA,
        tokenVaultB: poolAccount.tokenVaultB,
        lpMint: poolAccount.lpMint,
        user: payer.publicKey,
        userTokenAccountA: userTokenAccountA,
        userTokenAccountB: userTokenAccountB,
        userLpTokenAccount: userLpTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nTransaction:", sig);
    console.log("Explorer:", explorerLink(sig));

    // Balances after
    const userA_after = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_after = await getAccount(
      provider.connection,
      userTokenAccountB
    );
    const userLp_after = await getAccount(
      provider.connection,
      userLpTokenAccount
    );
    const vaultA_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );

    console.log("\nAfter:");
    console.log("  User Token A:", formatTokens(userA_after.amount));
    console.log("  User Token B:", formatTokens(userB_after.amount));
    console.log("  User LP Tokens:", formatTokens(userLp_after.amount));
    console.log("  Vault A:", formatTokens(vaultA_after.amount));
    console.log("  Vault B:", formatTokens(vaultB_after.amount));

    const expectedLpAmount = new anchor.BN(100_000_000);
    assert.ok(
      new anchor.BN(userLp_after.amount).eq(expectedLpAmount),
      `LP amount should be ${expectedLpAmount}`
    );
  });

  it("Adds more liquidity, respecting the pool ratio", async () => {
    console.log("\n--- Test 3: Add More Liquidity ---");

    const amountA = new anchor.BN(50_000_000);
    const amountB = new anchor.BN(50_000_000);

    console.log("Depositing:", formatTokens(amountA.toNumber()), "Token A");
    console.log("Depositing:", formatTokens(amountB.toNumber()), "Token B");

    const poolAccount = await program.account.pool.fetch(poolPda);
    const userLpTokenAccount = await getAssociatedTokenAddress(
      poolAccount.lpMint,
      payer.publicKey
    );

    const vaultA_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );
    const userLp_before = await getAccount(
      provider.connection,
      userLpTokenAccount
    );

    console.log("\nBefore:");
    console.log("  Vault A:", formatTokens(vaultA_before.amount));
    console.log("  Vault B:", formatTokens(vaultB_before.amount));
    console.log("  User LP:", formatTokens(userLp_before.amount));

    const sig = await program.methods
      .addLiquidity(amountA, amountB)
      .accountsStrict({
        pool: poolPda,
        mintA: poolAccount.mintA,
        mintB: poolAccount.mintB,
        tokenVaultA: poolAccount.tokenVaultA,
        tokenVaultB: poolAccount.tokenVaultB,
        lpMint: poolAccount.lpMint,
        user: payer.publicKey,
        userTokenAccountA: userTokenAccountA,
        userTokenAccountB: userTokenAccountB,
        userLpTokenAccount: userLpTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nTransaction:", sig);
    console.log("Explorer:", explorerLink(sig));

    const vaultA_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );
    const userLp_after = await getAccount(
      provider.connection,
      userLpTokenAccount
    );

    console.log("\nAfter:");
    console.log("  Vault A:", formatTokens(vaultA_after.amount));
    console.log("  Vault B:", formatTokens(vaultB_after.amount));
    console.log("  User LP:", formatTokens(userLp_after.amount));

    assert.equal(Number(vaultA_after.amount), 150_000_000);
    assert.equal(Number(vaultB_after.amount), 150_000_000);

    const expectedLpTotal = new anchor.BN(150_000_000);
    assert.ok(
      new anchor.BN(userLp_after.amount).eq(expectedLpTotal),
      `Total LP should be ${expectedLpTotal}`
    );
  });

  it("Swaps token A for token B", async () => {
    console.log("\n--- Test 4: Swap A -> B ---");

    const amountIn = new anchor.BN(30_000_000);
    const minAmountOut = new anchor.BN(1);

    console.log("Swapping:", formatTokens(amountIn.toNumber()), "Token A");

    const poolAccount = await program.account.pool.fetch(poolPda);

    const userA_before = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_before = await getAccount(
      provider.connection,
      userTokenAccountB
    );
    const vaultA_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );

    console.log("\nBefore swap:");
    console.log("  User Token A:", formatTokens(userA_before.amount));
    console.log("  User Token B:", formatTokens(userB_before.amount));
    console.log("  Vault A:", formatTokens(vaultA_before.amount));
    console.log("  Vault B:", formatTokens(vaultB_before.amount));

    const sig = await program.methods
      .swap(amountIn, minAmountOut)
      .accountsStrict({
        pool: poolPda,
        mintA: poolAccount.mintA,
        mintB: poolAccount.mintB,
        tokenVaultA: poolAccount.tokenVaultA,
        tokenVaultB: poolAccount.tokenVaultB,
        user: payer.publicKey,
        userTokenAccountIn: userTokenAccountA,
        userTokenAccountOut: userTokenAccountB,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("\nTransaction:", sig);
    console.log("Explorer:", explorerLink(sig));

    const userA_after = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_after = await getAccount(
      provider.connection,
      userTokenAccountB
    );
    const vaultA_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );

    const amountOut = Number(userB_after.amount) - Number(userB_before.amount);

    console.log("\nAfter swap:");
    console.log("  User Token A:", formatTokens(userA_after.amount));
    console.log("  User Token B:", formatTokens(userB_after.amount));
    console.log("  Received:", formatTokens(amountOut), "Token B");
    console.log("  Vault A:", formatTokens(vaultA_after.amount));
    console.log("  Vault B:", formatTokens(vaultB_after.amount));

    assert.equal(
      Number(userA_before.amount) - Number(userA_after.amount),
      amountIn.toNumber(),
      "User A account balance change is incorrect"
    );

    assert.equal(
      Number(vaultA_after.amount) - Number(vaultA_before.amount),
      amountIn.toNumber(),
      "Vault A balance change is incorrect"
    );

    const expectedAmountOut = 24979163;
    assert.equal(
      Number(vaultB_before.amount) - Number(vaultB_after.amount),
      expectedAmountOut,
      "Vault B balance change is incorrect"
    );

    assert.equal(
      Number(userB_after.amount) - Number(userB_before.amount),
      expectedAmountOut,
      "User B account balance change is incorrect"
    );
  });

  it("Removes liquidity and returns proportional token amounts", async () => {
    console.log("\n--- Test 5: Remove Liquidity ---");

    const poolAccount = await program.account.pool.fetch(poolPda);
    const userLpTokenAccount = await getAssociatedTokenAddress(
      poolAccount.lpMint,
      payer.publicKey
    );

    const lpToBurn = new anchor.BN(50_000_000);
    console.log("Burning:", formatTokens(lpToBurn.toNumber()), "LP tokens");

    const vaultA_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_before = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );
    const userA_before = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_before = await getAccount(
      provider.connection,
      userTokenAccountB
    );
    const userLp_before = await getAccount(
      provider.connection,
      userLpTokenAccount
    );

    console.log("\nBefore:");
    console.log("  User LP:", formatTokens(userLp_before.amount));
    console.log("  Vault A:", formatTokens(vaultA_before.amount));
    console.log("  Vault B:", formatTokens(vaultB_before.amount));

    const sig = await program.methods
      .removeLiquidity(lpToBurn)
      .accountsStrict({
        pool: poolPda,
        mintA: poolAccount.mintA,
        mintB: poolAccount.mintB,
        tokenVaultA: poolAccount.tokenVaultA,
        tokenVaultB: poolAccount.tokenVaultB,
        lpMint: poolAccount.lpMint,
        user: payer.publicKey,
        userTokenAccountA: userTokenAccountA,
        userTokenAccountB: userTokenAccountB,
        userLpTokenAccount: userLpTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nTransaction:", sig);
    console.log("Explorer:", explorerLink(sig));

    const vaultA_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultA
    );
    const vaultB_after = await getAccount(
      provider.connection,
      poolAccount.tokenVaultB
    );
    const userA_after = await getAccount(
      provider.connection,
      userTokenAccountA
    );
    const userB_after = await getAccount(
      provider.connection,
      userTokenAccountB
    );
    const userLp_after = await getAccount(
      provider.connection,
      userLpTokenAccount
    );

    const receivedA = Number(userA_after.amount) - Number(userA_before.amount);
    const receivedB = Number(userB_after.amount) - Number(userB_before.amount);

    console.log("\nAfter:");
    console.log("  User LP:", formatTokens(userLp_after.amount));
    console.log("  Received Token A:", formatTokens(receivedA));
    console.log("  Received Token B:", formatTokens(receivedB));
    console.log("  Vault A:", formatTokens(vaultA_after.amount));
    console.log("  Vault B:", formatTokens(vaultB_after.amount));

    const expectedAmountA = 60_000_000;
    const expectedAmountB = 41_673_612;

    assert.equal(
      Number(vaultA_after.amount),
      Number(vaultA_before.amount) - expectedAmountA,
      `Vault A should decrease by ${expectedAmountA}`
    );
    assert.equal(
      Number(vaultB_after.amount),
      Number(vaultB_before.amount) - expectedAmountB,
      `Vault B should decrease by ${expectedAmountB}`
    );

    assert.equal(
      Number(userA_after.amount),
      Number(userA_before.amount) + expectedAmountA,
      `User token A should increase by ${expectedAmountA}`
    );
    assert.equal(
      Number(userB_after.amount),
      Number(userB_before.amount) + expectedAmountB,
      `User token B should increase by ${expectedAmountB}`
    );

    assert.equal(
      Number(userLp_after.amount),
      Number(userLp_before.amount) - lpToBurn.toNumber(),
      "User LP tokens should decrease by burned amount"
    );

    console.log("\n=== All tests passed ===");
  });
});
