import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { ProtectedMint } from "../target/types/protected_mint";

describe("protected-mint", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.ProtectedMint as Program<ProtectedMint>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
