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
    console.log("Bump: ", configAccountState.bump);
  });

  it("Releases funds to creator upon meeting threshold", async() => {
    
    console.log("Testing release funds to creator...");
    const creatorBalanceBefore = await provider.connection.getBalance(creator.publicKey) / LAMPORTS_PER_SOL
    console.log("Creator SOL balance before", creatorBalanceBefore);
    
    const [configAccountPDA, _] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("config-seed"),
        creator.publicKey.toBuffer(),
      ],
      program.programId
      );
    
    const configAccountBalanceBefore = await provider.connection.getBalance(configAccountPDA) / LAMPORTS_PER_SOL;
    console.log("Config Account SOL balance before", configAccountBalanceBefore);
    
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(configAccountPDA, 2000*LAMPORTS_PER_SOL),
      "confirmed"
    );

    const configAccountBalanceSimulatedMetThreshold = await provider.connection.getBalance(configAccountPDA) / LAMPORTS_PER_SOL;
    console.log("ConfigAccount post airdrop to simulate SOL receipts from Candy Machine", configAccountBalanceSimulatedMetThreshold);
    
    console.log("time before timeout", Date.now() / 1000);
    await new Promise(r => setTimeout(r, 11000));
    console.log("time now", Date.now() / 1000);

    await program.rpc.releaseFunds(
      {
        accounts: {
          creatorAddress: creator.publicKey,
          configAccount: configAccountPDA,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [creator]
    });
    
    const creatorBalanceAfter = await provider.connection.getBalance(creator.publicKey) / LAMPORTS_PER_SOL;
    console.log("Creator SOL balance after", creatorBalanceAfter);
    const configAccountBalanceAfter = await provider.connection.getBalance(configAccountPDA) / LAMPORTS_PER_SOL;
    console.log("Config Account SOL balance after", configAccountBalanceAfter);
    console.log("ConfigAccount Balance Change", configAccountBalanceSimulatedMetThreshold - configAccountBalanceBefore);
    console.log("Creator Balance Change", creatorBalanceAfter - creatorBalanceBefore);

    assert.ok(creatorBalanceAfter - creatorBalanceBefore == configAccountBalanceSimulatedMetThreshold - configAccountBalanceBefore);
    assert.ok(creatorBalanceAfter - creatorBalanceBefore == 2000);

    const configAccountState = await program.account.protectionConfig.fetch(configAccountPDA);
    assert.ok(configAccountState.thresholdMet == true);

  });

  it("Unathorized creator cannot release funds", async() => {
    //TODO
  });

  it("Processes refund for verified holder if threshold not met", async() => {
    //TODO
  });


  it("Denies refund for unverified holder", async() => {
    //TODO
  });


});
