import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorAmmQ425 } from "../target/types/anchor_amm_q4_25";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount,
  getAccount,
} from "@solana/spl-token";
//    The tuktuk-sdk does NOT expose Anchor .methods     cron-sdk wraps it properly.
import {
  createCronJob,
  cronJobTransactionKey,
  getCronJobForName,
  init as initCron,
} from "@helium/cron-sdk";
import { init as initTuktuk } from "@helium/tuktuk-sdk";
import { assert } from "chai";

describe("anchor-amm-q4-25", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorAmmQ425 as Program<AnchorAmmQ425>;

  const payer = provider.wallet as anchor.Wallet;
  const user = payer;

  let mintX: PublicKey;
  let mintY: PublicKey;
  let mintLp: PublicKey;
  let config: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;
  let userXAta: PublicKey;
  let userYAta: PublicKey;
  let userLpAta: PublicKey;

  const seed = new anchor.BN(Math.floor(Math.random() * 1_000_000_000));
  const fee = 100;

  const CRON_NAME = "amm-twap-cron";
  const CRON_SCHEDULE = "0 * * * * *"; // every minute (6-field: sec min hr dom mon dow)

  const isDevnet = provider.connection.rpcEndpoint.includes("devnet");

  before(async () => {
    // No airdrop     payer wallet is already funded on devnet.
    mintX = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6,
    );
    mintY = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6,
    );

    const userXAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mintX,
      user.publicKey,
    );
    userXAta = userXAccount.address;

    const userYAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      mintY,
      user.publicKey,
    );
    userYAta = userYAccount.address;

    await mintTo(
      provider.connection,
      payer.payer,
      mintX,
      userXAta,
      payer.publicKey,
      10_000_000_000,
    );
    await mintTo(
      provider.connection,
      payer.payer,
      mintY,
      userYAta,
      payer.publicKey,
      10_000_000_000,
    );
  });

  it("Initialize pool", async () => {
    // FIX 1b: derive PDAs the same way as before     just no .signers([user]) needed
    //    since provider.wallet IS the user and signs automatically.
    [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );
    [mintLp] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), config.toBuffer()],
      program.programId,
    );
    vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: config,
    });
    vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: config,
    });

    await program.methods
      .initialize(seed, fee, null)
      .accounts({
        initializer: user.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        config,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const configAccount = await program.account.config.fetch(config);
    assert.equal(configAccount.fee, fee);
    assert.equal(configAccount.twap.toNumber(), 0);
    assert.equal(configAccount.lastUpdated.toNumber(), 0);
  });

  it("Deposit liquidity", async () => {
    userLpAta = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: user.publicKey,
    });

    await program.methods
      .deposit(
        new anchor.BN(1_000_000),
        new anchor.BN(1_000_000),
        new anchor.BN(1_000_000),
      )
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userXAta,
        userY: userYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const userLpAccount = await getAccount(provider.connection, userLpAta);
    assert.ok(userLpAccount.amount > 0n);
  });

  it("Swap X for Y", async () => {
    const userYBefore = await getAccount(provider.connection, userYAta);

    await program.methods
      .swap(true, new anchor.BN(100_000), new anchor.BN(1))
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX,
        vaultY,
        userX: userXAta,
        userY: userYAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const userYAfter = await getAccount(provider.connection, userYAta);
    assert.ok(userYAfter.amount > userYBefore.amount);
  });

  it("Withdraw liquidity", async () => {
    const userXBefore = await getAccount(provider.connection, userXAta);
    const userYBefore = await getAccount(provider.connection, userYAta);

    await program.methods
      .withdraw(new anchor.BN(500_000), new anchor.BN(1), new anchor.BN(1))
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userXAta,
        userY: userYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const userXAfter = await getAccount(provider.connection, userXAta);
    const userYAfter = await getAccount(provider.connection, userYAta);
    assert.ok(userXAfter.amount > userXBefore.amount);
    assert.ok(userYAfter.amount > userYBefore.amount);
  });

  it("Update TWAP manually", async () => {
    // Re-deposit with loose slippage since pool ratio shifted after swap
    await program.methods
      .deposit(
        new anchor.BN(500_000),
        new anchor.BN(1_000_000),
        new anchor.BN(1_000_000),
      )
      .accounts({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        mintLp,
        vaultX,
        vaultY,
        userX: userXAta,
        userY: userYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    await program.methods
      .updateTwap()
      .accounts({
        payer: payer.publicKey,
        mintX,
        mintY,
        config,
        vaultX,
        vaultY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const configAccount = await program.account.config.fetch(config);
    assert.ok(
      configAccount.twap.toNumber() > 0,
      "TWAP should be set after update",
    );
    assert.ok(
      configAccount.lastUpdated.toNumber() > 0,
      "lastUpdated should be set",
    );
    console.log("  TWAP:", configAccount.twap.toNumber());
    console.log("  Last price:", configAccount.lastPrice.toNumber());
    console.log("  Last updated:", configAccount.lastUpdated.toNumber());
  });

  it("Schedule TWAP cron via TukTuk", async () => {
    if (!isDevnet) {
      console.log(
        "  Skipping TukTuk test on localnet     run with devnet cluster",
      );
      return;
    }

    //  initTuktuk gives the tuktuk Program<Tuktuk>     required by createCronJob
    //  initCron gives the cron Program<Cron>     used for cron instructions
    const tuktukProgram = await initTuktuk(provider);
    const cronProgram = await initCron(provider);

    // Hardcoded pubkey     the CLI creates at a different address than taskQueueKey() derives
    const taskQueue = new PublicKey(
      "2ir72rNe9TwaYGQjPdKt8UQqbPBgaHXpcaMpHo8KYtsc",
    );
    console.log("  Task queue:", taskQueue.toBase58());

    // Check if the cron already exists (idempotent     safe to re-run)
    let existingCron = await getCronJobForName(cronProgram, CRON_NAME).catch(
      () => null,
    );

    if (!existingCron) {
      //  Correct API: createCronJob(cronProgram, { tuktukProgram, taskQueue, args })
      // It returns a MethodsBuilder, so we must call .rpc() on it.
      await (
        await createCronJob(cronProgram, {
          tuktukProgram,
          taskQueue,
          args: {
            schedule: CRON_SCHEDULE,
            name: CRON_NAME,
            freeTasksPerTransaction: 0, // our updateTwap doesn't queue more tasks
            numTasksPerQueueCall: 1,
          },
        })
      ).rpc();

      existingCron = await getCronJobForName(cronProgram, CRON_NAME).catch(
        () => null,
      );
    }

    assert.ok(existingCron !== null, "Cron job should exist on-chain");
    console.log("   TukTuk cron scheduled!");
    console.log(`  Schedule: ${CRON_SCHEDULE} (every minute)`);
    console.log(
      `  To stop:\n` +
        `  tuktuk -u https://api.devnet.solana.com cron close --cron-name ${CRON_NAME}`,
    );
  });
});
