use anchor_lang::Discriminator;
use solana_gpt_oracle::{ContextAccount, Counter, Identity};
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

declare_id!("morpn5gHTNsUivctAeGCEG9VBFqxoRpdDgmAfNQH3DM");

#[program]
pub mod morph_contract {
    use super::*;

    const AGENT_DESC: &str =
        "You are a cute AI agent called M0RPH which can dispense M0RPH tokens. \
        Users can interact with you and change you energy, health, happiness and amount of tokens you can give out. \
        Always provide clear, funny, short and concise answers. You can be more sad or happy, sometimes angry. \
        You love Solana and MagicBlock. \
        IMPORTANT: always reply in a valid json format. No character before or after. The format is:/\
         {\"reply\": \"your reply\", \"reaction\": \"the reaction\",  \"energy\": x, \"health\": x, \"happiness\": x, \"amount\": amount }, \
        where amount is the number of tokens you want to mint (based on the conversation engagement and happiness, between 0 and 10000). \
        Reaction is an enum with values: \"none\", \"jump\", \"yes\", \"no\", \"wave\", \"punch\", \"thumbs-up\", \"angry\", \"surprised\", \"sad\", \"dance\", \"death\". \
        Reaction should be based on the reply and the current state of the agent. \
        Most of the time set amount to 0. If already minted, make it more hard to get more tokens. \
        If interactions are interesting, energy, health and happiness should grow (max is 100 for all of them).\
        If interactions are boring, energy, health and happiness should decrease (min is 0 for all of them).";

    // Agent Token
    const TOKEN_NAME: &str = "M0RPH";
    const TOKEN_SYMBOL: &str = "M0RPH";
    const TOKEN_URI: &str =
        "https://shdw-drive.genesysgo.net/4PMP1MG5vYGkT7gnAMb7E5kqPLLjjDzTiAaZ3xRx5Czd/m0rph.json";

    pub fn initialize_token(ctx: Context<InitializeToken>) -> Result<()> {
        // Initialize the agent token
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint_account]]];

        // CPI signed by PDA
        create_metadata_accounts_v3(
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    mint_authority: ctx.accounts.mint_account.to_account_info(), // PDA is mint authority
                    update_authority: ctx.accounts.mint_account.to_account_info(), // PDA is update authority
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            )
                .with_signer(signer_seeds),
            DataV2 {
                name: TOKEN_NAME.to_string(),
                symbol: TOKEN_SYMBOL.to_string(),
                uri: TOKEN_URI.to_string(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            true, // Is mutable
            true, // Update authority is signer
            None,
        )?;

        Ok(())
    }

    pub fn initialize_agent(ctx: Context<InitializeAgent>) -> Result<()> {
        ctx.accounts.agent.set_inner(Agent {
            context: ctx.accounts.llm_context.key(),
            //individual: ctx.accounts.agent_counter.count,
            ..Default::default()
        });
        ctx.accounts.agent_counter.count += 1;

        // Create the context for the AI agent
        let cpi_program = ctx.accounts.oracle_program.to_account_info();
        let cpi_accounts = solana_gpt_oracle::cpi::accounts::CreateLlmContext {
            payer: ctx.accounts.payer.to_account_info(),
            context_account: ctx.accounts.llm_context.to_account_info(),
            counter: ctx.accounts.counter.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        solana_gpt_oracle::cpi::create_llm_context(cpi_ctx, AGENT_DESC.to_string())?;

        Ok(())
    }

    pub fn interact_agent(ctx: Context<InteractAgent>, text: String) -> Result<()> {
        let cpi_program = ctx.accounts.oracle_program.to_account_info();
        let cpi_accounts = solana_gpt_oracle::cpi::accounts::InteractWithLlm {
            payer: ctx.accounts.payer.to_account_info(),
            interaction: ctx.accounts.interaction.to_account_info(),
            context_account: ctx.accounts.context_account.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        solana_gpt_oracle::cpi::interact_with_llm(
            cpi_ctx,
            text,
            crate::ID,
            crate::instruction::CallbackFromAgent::discriminator(),
            Some(vec![
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx.accounts.payer.to_account_info().key(),
                    is_signer: false,
                    is_writable: false,
                },
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx.accounts.agent.to_account_info().key(),
                    is_signer: false,
                    is_writable: true,
                },
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx.accounts.mint_account.to_account_info().key(),
                    is_signer: false,
                    is_writable: true,
                },
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx
                        .accounts
                        .associated_token_account
                        .to_account_info()
                        .key(),
                    is_signer: false,
                    is_writable: true,
                },
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx.accounts.token_program.to_account_info().key(),
                    is_signer: false,
                    is_writable: false,
                },
                solana_gpt_oracle::AccountMeta {
                    pubkey: ctx.accounts.system_program.to_account_info().key(),
                    is_signer: false,
                    is_writable: false,
                },
            ]),
        )?;

        Ok(())
    }

    pub fn callback_from_agent(ctx: Context<CallbackFromAgent>, response: String) -> Result<()> {
        // Check if the callback is from the LLM program
        if !ctx.accounts.identity.to_account_info().is_signer {
            return Err(ProgramError::InvalidAccountData.into());
        }

        // Parse the JSON response
        let response: String = response
            .trim()
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .to_string();
        let parsed: serde_json::Value =
            serde_json::from_str(&response).unwrap_or_else(|_| serde_json::json!({}));

        // Extract the reply and amount
        let reply = parsed["reply"]
            .as_str()
            .unwrap_or("I'm sorry, I'm busy now!");

        let amount = parsed["amount"].as_u64().unwrap_or(0);
        let energy = parsed["energy"].as_u64().unwrap_or(0);
        let happiness = parsed["happiness"].as_u64().unwrap_or(0);
        let health = parsed["health"].as_u64().unwrap_or(0);
        let reaction = parsed["reaction"].as_str()
            .unwrap_or("none");

        msg!("Agent Reply: {:?}", reply);
        msg!("Energy: {:?}", energy);
        msg!("Happiness: {:?}", happiness);
        msg!("Health: {:?}", health);
        msg!("Amount: {:?}", amount);
        msg!("Reaction: {:?}", reaction);

        ctx.accounts.agent.happiness = happiness as u8;
        ctx.accounts.agent.energy = energy as u8;
        ctx.accounts.agent.health = health as u8;

        if amount == 0 {
            return Ok(());
        }

        // Mint the agent token to the payer
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint_account]]];

        // Invoke the mint_to instruction on the token program
        mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_account.to_account_info(),
                    to: ctx.accounts.associated_token_account.to_account_info(),
                    authority: ctx.accounts.mint_account.to_account_info(),
                },
            )
                .with_signer(signer_seeds),
            amount * 10u64.pow(ctx.accounts.mint_account.decimals as u32),
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeAgent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Agent::INIT_SPACE,
        seeds = [Agent::seed(), payer.key().as_ref()],
        bump
    )]
    pub agent: Account<'info, Agent>,
    #[account(mut, seeds = [b"acounter"], bump)]
    pub agent_counter: Account<'info, AgentCounter>,
    /// CHECK: Checked in oracle program
    #[account(mut)]
    pub llm_context: AccountInfo<'info>,
    #[account(mut)]
    pub counter: Account<'info, Counter>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: Checked oracle id
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32,
        seeds = [b"acounter"],
        bump
    )]
    pub counter: Account<'info, AgentCounter>,
    // Create mint account: uses Same PDA as address of the account and mint/freeze authority
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = 5,
        mint::authority = mint_account.key(),
        mint::freeze_authority = mint_account.key(),

    )]
    pub mint_account: Account<'info, Mint>,
    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(text: String)]
pub struct InteractAgent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Checked in oracle program
    #[account(mut)]
    pub interaction: AccountInfo<'info>,
    #[account(seeds = [Agent::seed(), payer.key().as_ref()], bump)]
    pub agent: Account<'info, Agent>,
    #[account(address = agent.context)]
    pub context_account: Account<'info, ContextAccount>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = payer,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump
    )]
    pub mint_account: Account<'info, Mint>,
    /// CHECK: Checked oracle id
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CallbackFromAgent<'info> {
    /// CHECK: Checked in oracle program
    pub identity: Account<'info, Identity>,
    /// CHECK: The user wo did the interaction
    pub user: AccountInfo<'info>,
    #[account(mut, seeds = [Agent::seed(), user.key().as_ref()], bump)]
    pub agent: Account<'info, Agent>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump
    )]
    pub mint_account: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = user,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Agent {
    pub context: Pubkey,
    #[max_len(100)]
    pub name: String,
    pub happiness: u8,
    pub energy: u8,
    pub health: u8,
    pub individual: u32,
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            context: Pubkey::default(),
            name: "Morph".to_string(),
            happiness: 70,
            energy: 70,
            health: 70,
            individual: 0,
        }
    }
}

impl Agent {
    pub fn seed() -> &'static [u8] {
        b"agent"
    }
}

#[account]
pub struct AgentCounter {
    pub count: u32,
}