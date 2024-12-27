import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, BN, Program, web3 } from "@coral-xyz/anchor";
import { MorphContract } from "../target/types/morph_contract";

describe("morph-contract", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MorphContract as Program<MorphContract>;
  const gptOracleAddress = new web3.PublicKey(
    "LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab"
  );

  async function GetAgentAndInteraction(
    program: Program<MorphContract>,
    provider: AnchorProvider
  ) {
    const agentAddress = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("agent"), provider.wallet.publicKey.toBuffer()],
      program.programId
    )[0];

    const agent = await program.account.agent.fetch(agentAddress);

    // Interaction Address
    const interactionAddress = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("interaction"),
        provider.wallet.publicKey.toBuffer(),
        agent.context.toBuffer(),
      ],
      gptOracleAddress
    )[0];
    return { agentAddress, agent, interactionAddress };
  }

  it("InitializeToken!", async () => {
    const tx = await program.methods
      .initializeToken()
      .accounts({
        payer: provider.wallet.publicKey,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("InitializeAgent!", async () => {
    const counterAddress = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("counter")],
      gptOracleAddress
    )[0];

    const counter = await provider.connection.getAccountInfo(counterAddress);
    const count = new BN(counter.data.slice(8, 12), "le").toNumber();

    const contextAddress = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("context"), new BN(count).toArrayLike(Buffer, "le", 4)],
      gptOracleAddress
    )[0];

    const agentAddress = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("agent"), provider.wallet.publicKey.toBuffer()],
      program.programId
    )[0];

    const tx = await program.methods
      .initializeAgent()
      .accounts({
        payer: provider.wallet.publicKey,
        counter: counterAddress,
        llmContext: contextAddress,
        agent: agentAddress,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("InteractAgent!", async () => {
    const { agentAddress, agent, interactionAddress } =
      await GetAgentAndInteraction(program, provider);

    const tx = await program.methods
      .interactAgent("Can you give me some token?")
      .accounts({
        payer: provider.wallet.publicKey,
        interaction: interactionAddress,
        contextAccount: agent.context,
        agent: agentAddress,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });
});
