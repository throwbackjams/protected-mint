import * as anchor from "@project-serum/anchor";
import { Program, Provider, Wallet } from "@project-serum/anchor";
import { assert, config, expect } from "chai";
import { ProtectedMint } from "../target/types/protected_mint";
import {
  Keypair,
  Account,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import { Token, TOKEN_PROGRA_ID} from "@solana/spl-token";


describe("protected-mint", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ProtectedMint as Program<ProtectedMint>;
  
  console.log("Initializing environment...");
  let creator: Keypair;
  let thresholdLevel: number;
  let salesPrice: number;
  let maxQuantity: number;
  
  creator = Keypair.generate();
  console.log("Creator pubkey", creator.publicKey.toString());

  it("Config account initialized", async () => {
    console.log("Testing config account initialized...");
    //TODO
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(creator.publicKey, 10*LAMPORTS_PER_SOL),
      "confirmed"
    );

    salesPrice = 2 * LAMPORTS_PER_SOL;
    maxQuantity = 1000;
    thresholdLevel = maxQuantity * salesPrice;

    const nowBn = new anchor.BN(Date.now() / 1000);
    let endSalesTime = nowBn.add(new anchor.BN(10));

    const [configAccountPDA, _] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("config-seed"),
        creator.publicKey.toBuffer(),
      ],
      program.programId
    );
     
    await program.rpc.initializeConfig(
      new anchor.BN(thresholdLevel),
      endSalesTime,
      new anchor.BN(salesPrice),
      new anchor.BN(maxQuantity),
      {
        accounts: {
          creator: creator.publicKey,
          configAccount: configAccountPDA,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [creator]
      });

    const configAccountState = await program.account.protectionConfig.fetch(configAccountPDA);

    assert.ok(configAccountState.creatorAddress.toBase58() == creator.publicKey.toBase58());
    console.log("Creator address state", configAccountState.creatorAddress.toBase58());
    assert.ok(configAccountState.salePrice.toNumber() == salesPrice);
    console.log("Sales price:", configAccountState.salePrice.toNumber()/ LAMPORTS_PER_SOL);
    assert.ok(configAccountState.maxQuantity.toNumber() == maxQuantity);
    console.log("Max Quantity:", configAccountState.maxQuantity.toNumber());
    assert.ok(configAccountState.thresholdLevel.toNumber() == thresholdLevel);
    console.log("Threshold level: " + configAccountState.thresholdLevel.toNumber() / LAMPORTS_PER_SOL);
    assert.ok(configAccountState.thresholdMet == false);
    console.log("Threshold met:", configAccountState.thresholdMet);
    assert.ok(configAccountState.endSalesTime.toNumber() == endSalesTime.toNumber());
    console.log("End sales time state:", configAccountState.endSalesTime.toNumber());
  });

  it("Releases funds to creator upon meeting threshold", async() => {
    //TODO
  });

  it("Processes refund for verified holder if threshold not met", async() => {
    //TODO
  });

  it("Verifies release funds & refund restricted before end of sales time", async() => {
    //TODO
  });

  it("Denies refund for unverified holder", async() => {
    //TODO
  });

  it("Unathorized creator cannot release funds", async() => {
    //TODO
  });
});
