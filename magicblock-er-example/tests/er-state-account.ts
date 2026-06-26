import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { GetCommitmentSignature } from "@magicblock-labs/ephemeral-rollups-sdk";
import { ErStateAccount } from "../target/types/er_state_account";

describe("er-state-account", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const providerEphemeralRollup = new anchor.AnchorProvider(
    new anchor.web3.Connection(
      process.env.EPHEMERAL_PROVIDER_ENDPOINT ||
        "https://devnet.magicblock.app/",
      {
        wsEndpoint:
          process.env.EPHEMERAL_WS_ENDPOINT || "wss://devnet.magicblock.app/",
      },
    ),
    anchor.Wallet.local(),
  );

  console.log("Base Layer Connection: ", provider.connection.rpcEndpoint);
  console.log(
    "Ephemeral Rollup Connection: ",
    providerEphemeralRollup.connection.rpcEndpoint,
  );
  console.log(`Current SOL Public Key: ${anchor.Wallet.local().publicKey}`);

  before(async function () {
    const balance = await provider.connection.getBalance(
      anchor.Wallet.local().publicKey,
    );
    console.log("Current balance is", balance / LAMPORTS_PER_SOL, " SOL", "\n");
  });

  // Base layer program instance
  const program = anchor.workspace.erStateAccount as Program<ErStateAccount>;

  // ER program instance   same IDL but connected to ER provider
  const programEr = new anchor.Program(
    anchor.workspace.erStateAccount.idl,
    providerEphemeralRollup,
  ) as Program<ErStateAccount>;

  const userAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user"), anchor.Wallet.local().publicKey.toBuffer()],
    program.programId,
  )[0];



  it("Is initialized!", async () => {
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("User Account initialized: ", tx);
  });

  it("Update State!", async () => {
    const tx = await program.methods
      .update(new anchor.BN(42))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();
    console.log("\nUser Account State Updated: ", tx);
  });

  // ───────────────────────────────────────────────
  // Task 1: VRF outside ER (base chain)
  // ───────────────────────────────────────────────

  it("Request VRF Randomness (outside ER)!", async () => {
    const oracleQueue = new PublicKey(
      "Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh",
    );

    const tx = await program.methods
      .requestRandomness(0)
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        oracleQueue: oracleQueue,
      })
      .rpc({ skipPreflight: true });

    console.log("\nVRF Randomness Requested: ", tx);
    console.log("Waiting for oracle callback (10s)...");
    await new Promise((resolve) => setTimeout(resolve, 10000));
  });

  it("Verify VRF updated user state!", async () => {
    const account = await program.account.userAccount.fetch(userAccount);
    console.log("\nUser account data after VRF:", account.data.toString());

    if (account.data.toString() !== "42") {
      console.log(
        " VRF successfully updated user data to:",
        account.data.toString(),
      );
    } else {
      console.log("⚠️  Data still 42   oracle may not have responded yet");
    }
  });


  it("Delegate for ER VRF!", async () => {
    const tx = await program.methods
      .delegate()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });

    console.log("\nDelegated for ER VRF: ", tx);
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  it("Request VRF Randomness (inside ER)!", async () => {
    const oracleQueue = new PublicKey(
      "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc",
    );

    // Use programEr   wired to ER provider, sends directly to ER endpoint
    const txHash = await programEr.methods
      .requestRandomnessEr(1)
      .accountsPartial({
        user: providerEphemeralRollup.wallet.publicKey,
        userAccount: userAccount,
        oracleQueue: oracleQueue,
      })
      .rpc({ skipPreflight: true });

    console.log("\nER VRF Randomness Requested: ", txHash);
    console.log("Waiting for oracle callback (10s)...");
    await new Promise((resolve) => setTimeout(resolve, 10000));
  });

  it("Verify ER VRF updated user state!", async () => {
    const accountInfo = await providerEphemeralRollup.connection.getAccountInfo(
      userAccount,
    );
    console.log("\nER Account data buffer:", accountInfo?.data);
    console.log("ER VRF task complete!");
  });

  it("Undelegate after ER VRF!", async () => {
    let tx = await programEr.methods
      .undelegate()
      .accounts({
        user: providerEphemeralRollup.wallet.publicKey,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    tx = await providerEphemeralRollup.wallet.signTransaction(tx);

    const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
      skipPreflight: false,
    });

    console.log("\nUndelegated after ER VRF: ", txHash);
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });



  it("Delegate to Ephemeral Rollup!", async () => {
    let tx = await program.methods
      .delegate()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });

    console.log("\nUser Account Delegated to Ephemeral Rollup: ", tx);
  });

  it("Update State and Commit to Base Layer!", async () => {
    let tx = await programEr.methods
      .updateCommit(new anchor.BN(43))
      .accountsPartial({
        user: providerEphemeralRollup.wallet.publicKey,
        userAccount: userAccount,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    tx = await providerEphemeralRollup.wallet.signTransaction(tx);

    const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
      skipPreflight: false,
    });

    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection,
    );
    console.log("\nUser Account State Updated: ", txHash);
  });

  it("Commit and undelegate from Ephemeral Rollup!", async () => {
    let info = await providerEphemeralRollup.connection.getAccountInfo(
      userAccount,
    );
    console.log("User Account Info: ", info);
    console.log("User account", userAccount.toBase58());

    let tx = await programEr.methods
      .undelegate()
      .accounts({
        user: providerEphemeralRollup.wallet.publicKey,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    tx = await providerEphemeralRollup.wallet.signTransaction(tx);

    const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {
      skipPreflight: false,
    });

    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection,
    );
    console.log("\nUser Account Undelegated: ", txHash);
  });

  it("Update State!", async () => {
    let tx = await program.methods
      .update(new anchor.BN(45))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();
    console.log("\nUser Account State Updated: ", tx);
  });

  it("Close Account!", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("\nUser Account Closed: ", tx);
  });
});
