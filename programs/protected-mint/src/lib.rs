use anchor_lang::prelude::*;

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
        
        //Check if the Config Account's SOL balance has crossed the pre-determined threshold
        let available_lamports = **config_account.to_account_info().lamports.borrow(); 

        if !((config_account.threshold_level < available_lamports) || (config_account.threshold_met)) {
            return Err(ErrorCode::ThresholdNotMet.into());
        }

        //Release available lamports less rent exemption (in case more mints occur after indicated time)
        //Note: can also config such that end_sales_time == candy machine's end time. If so, then can transfer all lamports and close account below
        let creator_account = &ctx.accounts.creator_address;
        let space = 32 + 8 + 8 + 8 + 1 + 8 as usize; //Config Account: space = creator pubkey + sale price + quantity + threshold level + threshold bool + end sales time
        let rent = Rent::get()?;
        let rent_exempt_lamports = rent.minimum_balance(space);
        let lamports_to_transfer = available_lamports.checked_sub(rent_exempt_lamports);

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
                lamports_to_transfer.unwrap(),
            ),
            &[
                config_account.to_account_info().clone(),
                creator_account.to_account_info().clone(),
            ],
            &[authority_seeds],

        )?;


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
