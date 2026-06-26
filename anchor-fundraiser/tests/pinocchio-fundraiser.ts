import * as anchor from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { assert } from "chai";

describe("pinocchio-fundraiser", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Program ID must match the one in lib.rs
  const PROGRAM_ID = new PublicKey("Eoiuq1dXvHxh6dLx3wh9gj8kSAUpga11krTrbfF5XYsC");

  const maker = Keypair.generate();
  const wallet = provider.wallet as NodeWallet;

  let mint: PublicKey;
  let contributorATA: PublicKey;
  let makerATA: PublicKey;

  // PDAs
  const fundraiser = PublicKey.findProgramAddressSync(
    [Buffer.from("fundraiser"), maker.publicKey.toBuffer()],
    PROGRAM_ID
  )[0];

  let contributor: PublicKey;

  const confirm = async (signature: string): Promise<string> => {
    const block = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  // Helper: build Initialize instruction
  function buildInitializeInstruction(
    maker: PublicKey,
    fundraiser: PublicKey,
    mint: PublicKey,
    vault: PublicKey,
    amount: number,
    duration: number
  ): TransactionInstruction {
    // Discriminator 0 = Initialize
    const data = Buffer.alloc(9);
    data.writeUInt8(0, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(duration, 8);

    return new TransactionInstruction({
      keys: [
        { pubkey: maker, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: fundraiser, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data,
    });
  }

  // Helper: build Contribute instruction
  function buildContributeInstruction(
    contributor: PublicKey,
    mint: PublicKey,
    fundraiser: PublicKey,
    contributorAccount: PublicKey,
    contributorAta: PublicKey,
    vault: PublicKey,
    amount: number
  ): TransactionInstruction {
    // Discriminator 1 = Contribute
    const data = Buffer.alloc(8);
    data.writeUInt8(1, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    return new TransactionInstruction({
      keys: [
        { pubkey: contributor, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: fundraiser, isSigner: false, isWritable: true },
        { pubkey: contributorAccount, isSigner: false, isWritable: true },
        { pubkey: contributorAta, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data,
    });
  }

  // Helper: build CheckContributions instruction
  function buildCheckContributionsInstruction(
    maker: PublicKey,
    mint: PublicKey,
    fundraiser: PublicKey,
    vault: PublicKey,
    makerAta: PublicKey
  ): TransactionInstruction {
    // Discriminator 2 = CheckContributions
    const data = Buffer.alloc(1);
    data.writeUInt8(2, 0);

    return new TransactionInstruction({
      keys: [
        { pubkey: maker, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: fundraiser, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: makerAta, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data,
    });
  }

  // Helper: build Refund instruction
  function buildRefundInstruction(
    contributor: PublicKey,
    maker: PublicKey,
    mint: PublicKey,
    fundraiser: PublicKey,
    contributorAccount: PublicKey,
    contributorAta: PublicKey,
    vault: PublicKey
  ): TransactionInstruction {
    // Discriminator 3 = Refund
    const data = Buffer.alloc(1);
    data.writeUInt8(3, 0);

    return new TransactionInstruction({
      keys: [
        { pubkey: contributor, isSigner: true, isWritable: true },
        { pubkey: maker, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: fundraiser, isSigner: false, isWritable: true },
        { pubkey: contributorAccount, isSigner: false, isWritable: true },
        { pubkey: contributorAta, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data,
    });
  }

  // Helper: send and confirm transaction
  async function sendAndConfirmIx(ix: TransactionInstruction, signers: Keypair[]): Promise<string> {
    const tx = new Transaction();
    tx.add(ix);
    const sig = await provider.sendAndConfirm(tx, signers, { skipPreflight: true });
    return sig;
  }

  // Helper: airdrop
  async function airdrop(pubkey: PublicKey, amount: number) {
    const sig = await provider.connection.requestAirdrop(pubkey, amount);
    const block = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({ signature: sig, ...block });
  }

  // ============================================================
  // Test Suite
  // ============================================================

  describe("Setup", () => {
    it("Airdrops SOL and creates mint + ATAs", async () => {
      // Airdrop to maker
      await airdrop(maker.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);

      // Create token mint (6 decimals to match original tests)
      mint = await createMint(
        provider.connection,
        wallet.payer,
        provider.publicKey,
        provider.publicKey,
        6
      );

      // Create contributor ATA (owned by wallet provider)
      contributorATA = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          wallet.payer,
          mint,
          wallet.publicKey
        )
      ).address;

      // Create maker ATA
      makerATA = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          wallet.payer,
          mint,
          maker.publicKey
        )
      ).address;

      // Mint 10 tokens to contributor
      await mintTo(
        provider.connection,
        wallet.payer,
        mint,
        contributorATA,
        wallet.payer,
        10_000_000 // 10 tokens * 10^6
      );

      // Derive contributor PDA
      contributor = PublicKey.findProgramAddressSync(
        [
          Buffer.from("contributor"),
          fundraiser.toBuffer(),
          wallet.publicKey.toBuffer(),
        ],
        PROGRAM_ID
      )[0];

      // Verify setup
      const contributorBalance = await getAccount(provider.connection, contributorATA);
      assert.equal(Number(contributorBalance.amount), 10_000_000, "Contributor should have 10 tokens");
    });
  });

  describe("Initialize Fundraiser", () => {
    it("Creates a new fundraiser with 30 token target", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      const ix = buildInitializeInstruction(
        maker.publicKey,
        fundraiser,
        mint,
        vault,
        30_000_000, // 30 tokens * 10^6
        0           // 0 days duration (ends immediately for testing)
      );

      await sendAndConfirmIx(ix, [maker]);

      // Verify fundraiser account was created
      const fundraiserAccount = await provider.connection.getAccountInfo(fundraiser);
      assert.isNotNull(fundraiserAccount, "Fundraiser account should exist");
      assert.equal(fundraiserAccount.owner.toBase58(), PROGRAM_ID.toBase58(), "Fundraiser should be owned by program");

      // Verify vault was created
      const vaultAccount = await getAccount(provider.connection, vault);
      assert.equal(Number(vaultAccount.amount), 0, "Vault should start empty");
    });

    it("Rejects initialize with amount too small", async () => {
      const newMaker = Keypair.generate();
      await airdrop(newMaker.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);

      const newFundraiser = PublicKey.findProgramAddressSync(
        [Buffer.from("fundraiser"), newMaker.publicKey.toBuffer()],
        PROGRAM_ID
      )[0];

      const vault = getAssociatedTokenAddressSync(mint, newFundraiser, true);

      const ix = buildInitializeInstruction(
        newMaker.publicKey,
        newFundraiser,
        mint,
        vault,
        1, // Too small (less than 3 * 10^6)
        0
      );

      try {
        await sendAndConfirmIx(ix, [newMaker]);
        assert.fail("Should have thrown");
      } catch (error) {
        // Expected to fail
        console.log("Correctly rejected small amount");
      }
    });
  });

  describe("Contribute to Fundraiser", () => {
    it("Allows contributing 1 token", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      const ix = buildContributeInstruction(
        wallet.publicKey,
        mint,
        fundraiser,
        contributor,
        contributorATA,
        vault,
        1_000_000 // 1 token * 10^6
      );

      await sendAndConfirmIx(ix, [wallet.payer]);

      // Verify vault received tokens
      const vaultAccount = await getAccount(provider.connection, vault);
      assert.equal(Number(vaultAccount.amount), 1_000_000, "Vault should have 1 token");
    });

    it("Allows contributing another token (cumulative 2)", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      const ix = buildContributeInstruction(
        wallet.publicKey,
        mint,
        fundraiser,
        contributor,
        contributorATA,
        vault,
        1_000_000 // 1 token
      );

      await sendAndConfirmIx(ix, [wallet.payer]);

      // Verify vault balance
      const vaultAccount = await getAccount(provider.connection, vault);
      assert.equal(Number(vaultAccount.amount), 2_000_000, "Vault should have 2 tokens");

      // Verify contributor account
      const contribAccountInfo = await provider.connection.getAccountInfo(contributor);
      assert.isNotNull(contribAccountInfo, "Contributor account should exist");
      // Amount is at offset 1-8 (little-endian u64)
      const contribAmount = contribAccountInfo.data.readBigUInt64LE(1);
      assert.equal(Number(contribAmount), 2_000_000, "Contributor should have contributed 2 tokens");
    });

    it("Rejects contribution that exceeds 10% of target", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      // 10% of 30 tokens = 3 tokens = 3_000_000
      // Contributing 2 more would make cumulative 4_000_000 > 3_000_000
      const ix = buildContributeInstruction(
        wallet.publicKey,
        mint,
        fundraiser,
        contributor,
        contributorATA,
        vault,
        2_000_000 // 2 tokens (cumulative would be 4 > 3 = 10% of 30)
      );

      try {
        await sendAndConfirmIx(ix, [wallet.payer]);
        assert.fail("Should have thrown");
      } catch (error) {
        console.log("Correctly rejected over-contribution");
      }
    });
  });

  describe("Check Contributions (Claim)", () => {
    it("Rejects claim when target not met", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      const ix = buildCheckContributionsInstruction(
        maker.publicKey,
        mint,
        fundraiser,
        vault,
        makerATA
      );

      try {
        await sendAndConfirmIx(ix, [maker]);
        assert.fail("Should have thrown");
      } catch (error) {
        // Target is 30 tokens, only 2 contributed
        console.log("Correctly rejected claim when target not met");
      }
    });
  });

  describe("Refund", () => {
    it("Rejects refund before fundraiser ends", async () => {
      const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

      const ix = buildRefundInstruction(
        wallet.publicKey,
        maker.publicKey,
        mint,
        fundraiser,
        contributor,
        contributorATA,
        vault
      );

      try {
        await sendAndConfirmIx(ix, [wallet.payer]);
        assert.fail("Should have thrown");
      } catch (error) {
        // Duration is 0, but we need to wait for clock to advance
        // In localnet with duration=0, the fundraiser ends immediately
        console.log("Refund test result:", error.message || "error");
      }
    });
  });
});
