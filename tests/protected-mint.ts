import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { assert } from "chai";
import { ProtectedMint } from "../target/types/protected_mint";

describe("protected-mint", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.ProtectedMint as Program<ProtectedMint>;

  it("Config account initialized", async () => {
    //TODO
    assert.ok(true == false)
  });

  it("Releases funds to creator upon meeting threshold", async() => {
    //TODO
    assert.ok(true == false)
  });

  it("Processes refund for verified holder if threshold not met", async() => {
    //TODO
    assert.ok(true == false)
  });

  it("Verifies release funds & refund restricted before end of sales time", async() => {
    //TODO
    assert.ok(true == false)
  });

  it("Denies refund for unverified holder", async() => {
    //TODO
    assert.ok(true == false)
  });

  it("Unathorized creator cannot release funds", async() => {
    //TODO
    assert.ok(true == false)
  });

});
