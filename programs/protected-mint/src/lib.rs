use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Token, Mint, Burn};
use mpl_token_metadata;
use mpl_token_metadata::state::{Metadata, PREFIX, EDITION};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod protected_mint {
    use super::*;

    #[access_control(future_end_time(&ctx, end_sales_time))]
    pub fn initialize_config(
        ctx: Context<InitProtectionConfig>,
        threshold_level: u64,
        end_sales_time: i64,
        sale_price: u64,
        max_quantity: u64,
    ) -> Result<()> {
        let config_account = &mut ctx.accounts.config_account;

        //Set the parameters of the Config account
        config_account.creator_address = *ctx.accounts.creator.key;
        config_account.sale_price = sale_price;
        config_account.max_quantity = max_quantity;
        
        //Check that the provided threshold level does not exceed the total possible proceeds
        if threshold_level > sale_price * max_quantity {
            return Err(ErrorCode::ThresholdTooGreat.into());
        }
        
        //Note: Threshold level should be in number of lamports
        config_account.threshold_level = threshold_level;
        config_account.threshold_met = false;

        //Note: Candy Machine Treasury account address should be set to this PDA
        msg!("Config Account address is: {:?}",config_account.to_account_info().key);
        
        Ok(())

    }

    #[access_control(past_end_sales_time(&ctx.accounts.config_account, &ctx.accounts.clock))]
    pub fn release_funds(
        ctx: Context<ReleaseFunds>
    ) -> Result<()> {

        let config_account = &mut ctx.accounts.config_account;
        
        let available_lamports = **config_account.to_account_info().lamports.borrow(); 

        //Release available lamports less rent exemption (in case more mints occur after indicated time)
        //Note: can also config such that end_sales_time == candy machine's end time. If so, then can transfer all lamports and close account below
        let space = 32 + 8 + 8 + 8 + 1 + 8 as usize; //Config Account: space = creator pubkey + sale price + quantity + threshold level + threshold bool + end sales time
        let rent = Rent::get()?;
        let rent_exempt_lamports = rent.minimum_balance(space);
        let lamports_to_transfer = available_lamports.checked_sub(rent_exempt_lamports).unwrap();
        
        //Check if the Config Account's available SOL balance has crossed the pre-determined threshold or if the threshold_met boolean is true
        if !((config_account.threshold_level <= lamports_to_transfer) || (config_account.threshold_met)) {
            return Err(ErrorCode::ThresholdNotMet.into());
        }

        let creator_account = &ctx.accounts.creator_address;

        let (_config_account, config_account_bump) =
            Pubkey::find_program_address(&[b"config-seed".as_ref(), ctx.accounts.creator_address.key.as_ref()], ctx.program_id);

        let authority_seeds = &[
            b"config-seed".as_ref(), 
            ctx.accounts.creator_address.key.as_ref(), 
            &[config_account_bump]];

        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::system_instruction::transfer(
                config_account.to_account_info().key,
                &config_account.creator_address,
                lamports_to_transfer,
            ),
            &[
                config_account.to_account_info().clone(),
                creator_account.to_account_info().clone(),
            ],
            &[authority_seeds],

        )?;

        //Question: Is the PDA paying for the transfer transaction fee? Does this need to be accounted for in the avilable_lamports / lamports_to_transfer vars?

        // Note: Tried to use new Anchor wrappers functions for system program cpi calls but says Transfer not found in anchor_lang::system_program
        // let cpi_accounts = anchor_lang::system_program::Transfer{
        //     from: config_account.to_account_info(),
        //     to: config_account.creator_address.to_account_info(),
        // };

        // let cpi_context = CpiContext::new(system_program.to_account_info(), cpi_accounts);
        // anchor_lang::system_program::transfer(cpi_context.with_signer(&[authority_seeds]), lamports_to_transfer);

        // fn into_transfer_context(
        //     &self,
        // ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        //     let cpi_accounts = Transfer {
        //         from: config_account.to_account_info().clone(),
        //         to: config_account.creator_address.to_account_info().clone(),
        //     };
        //     let cpi_context = CpiContext::new(system_program.to_account_info(), cpi_accounts);
            
            
        // }

        config_account.threshold_met = true;

        Ok(())
    }

    #[access_control(past_end_sales_time(&ctx.accounts.config_account, &ctx.accounts.clock))]
    pub fn provide_refund(
        ctx: Context<RequestRefund>
    ) -> Result<()> {

        //Check if the threshold level is met & if account has sufficient lamports to transfer
        //TODO: Turn threshold check logic in provide_refund and release_funds into a function implemented on both structs
        let config_account = &mut ctx.accounts.config_account;
        
        let available_lamports = **config_account.to_account_info().lamports.borrow(); 

        //Available lamports excludes rent exemption
        let space = 32 + 8 + 8 + 8 + 1 + 8 as usize; //Config Account: space = creator pubkey + sale price + quantity + threshold level + threshold bool + end sales time
        let rent = Rent::get()?;
        let rent_exempt_lamports = rent.minimum_balance(space);
        let lamports_to_transfer = available_lamports.checked_sub(rent_exempt_lamports).unwrap();
        
        //Check if the Config Account's available SOL balance has crossed the pre-determined threshold or if the threshold_met boolean is true
        if !((config_account.threshold_level < lamports_to_transfer) || (config_account.threshold_met)) {
            return Err(ErrorCode::ThresholdNotMet.into());
        }

        if !(config_account.sale_price <= lamports_to_transfer) {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        //Check if signer user holds an NFT account, whose balance is 1, and whose token metadata updateAuthority matches the creator address in the Config
        let nft_token_account = &ctx.accounts.nft_token_account;
        let user = &ctx.accounts.user;
        let nft_mint_account = &ctx.accounts.nft_mint;

        //Check: token account is owned by the signer
        assert_eq!(nft_token_account.owner, user.key());
        //Check: token account mint corresponds to the mint account passed in
        assert_eq!(nft_token_account.mint, nft_mint_account.key());
        //Check: token account has quantity 1 (not empty)
        assert_eq!(nft_token_account.amount, 1);

        //Expect a Metaplex Master Edition so we derive the master_edition_key and compare to the mint passed to the program
        let master_edition_seed = &[
            PREFIX.as_bytes(),
            ctx.accounts.token_metadata_program.key.as_ref(),
            nft_token_account.mint.as_ref(),
            EDITION.as_bytes()
        ];

        let (master_edition_key, _master_edition_seed) =
            Pubkey::find_program_address(master_edition_seed, ctx.accounts.token_metadata_program.key);

        assert_eq!(master_edition_key, ctx.accounts.nft_mint.key());

        //Check that the metadata account derived from the passed-in NFT mint account matches the passed-in metadata account
        let nft_metadata_account = &ctx.accounts.nft_metadata_account;
        let nft_mint_account_pubkey = &ctx.accounts.nft_mint.key();

        let metadata_seed = &[
            "metadata".as_bytes(),
            ctx.accounts.token_metadata_program.key.as_ref(),
            nft_mint_account_pubkey.as_ref(),
        ];

        let (metadata_derived_key, _bump_seed) = 
            Pubkey::find_program_address(metadata_seed, ctx.accounts.token_metadata_program.key);
        
        assert_eq!(metadata_derived_key, nft_metadata_account.key());

        if ctx.accounts.nft_metadata_account.data_is_empty(){
            return Err(ErrorCode::NFTMetadataEmpty.into());
        }

        let metadata_full_account = &mut Metadata::from_account_info(&ctx.accounts.nft_metadata_account)?;

        let full_metadata_clone = metadata_full_account.clone();

        let expected_creator = config_account.creator_address;

        //Check that the updateAuthority field on the NFT metadata matches the creator of the ProtectionConfig Account
        //Assumes that the creator address in the second position is the main address (since the first address is the Candy Machine per Metaplex Docs)
        //Note: In a production version, better to check for "verified" creator and ensure projects verify their collections
        assert_eq!(
            full_metadata_clone.data.creators.as_ref().unwrap()[1].address,
            expected_creator
        );

        //User signer burns the NFT?
        let nft_mint = &ctx.accounts.nft_mint;
        
        let burn_cpi_accounts = Burn {
            to: nft_token_account.to_account_info().clone(),
            mint: nft_mint.to_account_info().clone(),
            authority: user.clone(),
        };

        let token_program = ctx.accounts.token_program.to_account_info();

        let burn_cpi_context = CpiContext::new(token_program, burn_cpi_accounts);

        token::burn(
            burn_cpi_context,
            1 as u64,
        )?;
        
        //Derive config account PDA and transfer sales_price to the user signer

        let (_config_account, config_account_bump) =
            Pubkey::find_program_address(&[b"config-seed".as_ref(), expected_creator.as_ref()], ctx.program_id);

        let authority_seeds = &[
            b"config-seed".as_ref(), 
            expected_creator.as_ref(), 
            &[config_account_bump]];
        
        let sales_price = config_account.sale_price;

        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::system_instruction::transfer(
                config_account.to_account_info().key,
                ctx.accounts.user.key,
                sales_price,
            ),
            &[
                config_account.to_account_info().clone(),
                ctx.accounts.user.clone(),
            ],
            &[authority_seeds],

        )?;

        Ok(())
    }

}

#[derive(Accounts)]
pub struct InitProtectionConfig<'info> {
    #[account(
        init,
        seeds = [
            b"config-seed".as_ref(),
            creator.key.as_ref()
        ],
        bump,
        payer = creator,
        space = 32 + 8 + 8 + 8 + 1 + 8
        //space = creator pubkey + sale price + quantity + threshold level + threshold bool + end sales time
    )]
    pub config_account: Account<'info, ProtectionConfig>,
    #[account(mut, signer)]
    /// CHECK: This is not dangerous because any one can create a ProtectionConfig
    pub creator: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>, 
}

#[derive(Accounts)]
pub struct ReleaseFunds<'info>{
    #[account(mut, signer)]
    /// CHECK: Already checked as signer?
    pub creator_address: AccountInfo<'info>,
    #[account(has_one = creator_address)] //check that the config_account passed in corresponds to the creator
    pub config_account: Account<'info, ProtectionConfig>,
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestRefund<'info>{
    #[account(mut, signer)]
    /// CHECK: Already checked as signer?
    pub user: AccountInfo<'info>,
    pub config_account: Account<'info, ProtectionConfig>,
    pub nft_mint: Account<'info, Mint>,
    pub nft_token_account: Account<'info, TokenAccount>,
    /// CHECK: Refund function will derive the metadata key and ensure matches nft_metadata_account
    pub nft_metadata_account: AccountInfo<'info>,
    #[account(address = mpl_token_metadata::ID)]
    /// CHECK: Already checked for address match?
    pub token_metadata_program: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct ProtectionConfig {
    creator_address: Pubkey,
    sale_price: u64,
    max_quantity: u64,
    threshold_level: u64,
    threshold_met: bool,
    end_sales_time: i64,
}

#[error_code]
pub enum ErrorCode{
    #[msg("End of sales time must start in the future")]
    EndSalesFuture,
    #[msg("Threshold level is greater than the product of sales_price and max_quantity")]
    ThresholdTooGreat,
    #[msg("Cannot trigger release of funds prior to end of designated sales window")]
    SaleNotOver,
    #[msg("Pre-determined protection threshold is not yet met. Unable to trigger release of funds")]
    ThresholdNotMet,
    #[msg("Insufficient Funds in Protection Vault. Unable to process refund")]
    InsufficientFunds,
    #[msg("NFT Metadata Account is empty")]
    NFTMetadataEmpty,
}

fn future_end_time<'info>(ctx: &Context<InitProtectionConfig<'info>>, end_sales_time: i64) -> Result<()> {
    if !(ctx.accounts.clock.unix_timestamp < end_sales_time) {
        return Err(ErrorCode::EndSalesFuture.into());
    }
    Ok(())
}

fn past_end_sales_time<'info>(
    config_account: &Account<'info, ProtectionConfig>,
    clock: &Sysvar<'info, Clock>,
 ) -> Result<()>{
    if !(config_account.end_sales_time < clock.unix_timestamp) {
        return Err(ErrorCode::SaleNotOver.into());
    }
    Ok(())
}
