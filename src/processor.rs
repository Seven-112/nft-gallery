use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        program::{invoke, invoke_signed},
        program_pack::Pack   
    },
    borsh::{BorshDeserialize, BorshSerialize},
    spl_token::state::Account as TokenAccount
};
use spl_token_metadata::{
    state::{Metadata}
};

use crate::{
    error::HeroError, 
    instruction::HeroInstruction,
    state:: {
        NFTRecord,
        NFT_RECORD_SIZE
    }
};
use std::str::FromStr;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AddRecordArgs {
    pub hero_id: u8,
    pub content_uri: String,
    pub key_nft: String,
    pub last_price: u64,
    pub listed_price: u64
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct UpdateRecordArgs {
    pub hero_id: u8,
    pub key_nft: Pubkey,
    pub new_price: u64,
    pub content_uri: String
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BuyRecordArgs {
    pub hero_id: u8,
}


pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // unpack instruction_data and get proper instruction
        let instruction = HeroInstruction::unpack(instruction_data)?;
        match instruction {
            HeroInstruction::AddRecord(args) => {
                msg!("Instruction: AddRecord");
                Self::process_add_record(accounts, &args, program_id)
            },
            HeroInstruction::UpdateRecord(args) => {
                msg!("Instruction: UpdateRecord");
                Self::process_update_record(accounts, &args, program_id)
            },
            HeroInstruction::BuyRecord(args) => {
                msg!("Instruction: BuyRecord");
                Self::process_buy_record(accounts, &args, program_id)
            }
        }
    }

    /// Add seats to our repository account. 
    /// Now Seat Count is limited to 20. It can be expanded further.
    /// 1. we need to approve pda to delegate seat.
    /// 2. add record to our repository.
    /// 
    fn process_add_record(
        accounts: &[AccountInfo],
        args: &AddRecordArgs,
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        // account who adds hero
        let adder_account = next_account_info(account_info_iter)?;
        if !adder_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        // account which we will save all hero informations
        let repository_account = next_account_info(account_info_iter)?;
        if repository_account.owner != program_id {
            msg!("Derived account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }

        // associated token account of hero mint token address
        let associated_token_account = next_account_info(account_info_iter)?;
        msg!("associated_token_account ={:?}", associated_token_account);
        
        // get pda to get approved from associated token account
        // later, we would use pda to send nft to others.
        let (pda, _nonce) = Pubkey::find_program_address(&[b"hallofheros"], program_id);
        let pda_account = next_account_info(account_info_iter)?;
        
        let token_program = next_account_info(account_info_iter)?;
        msg!("token_program ={:?}", token_program);

        // approve
        let approve_ix = spl_token::instruction::approve(
            token_program.key,
            associated_token_account.key,
            &pda,
            adder_account.key,
            &[&adder_account.key],
            1
        )?;
        invoke(
            &approve_ix,
            &[
                associated_token_account.clone(),
                pda_account.clone(),
                adder_account.clone(),
                token_program.clone(),
            ],
        )?;

        // save new nft record to our repository
        let nft_record = NFTRecord {
            hero_id: args.hero_id,
            content_uri: args.content_uri.to_string(),
            key_nft: Pubkey::from_str(&args.key_nft).unwrap(),
            last_price: args.last_price,
            listed_price: args.listed_price
        };
        Self::save_nft_data_to_repository(&nft_record, repository_account.clone())?;

        Ok(())
    }

    /// users can change content_uri and price of hero
    /// so we need to update record
    /// 1. verify ownership of nft(seat)
    /// 2. update record
    /// 
    fn process_update_record(
        accounts: &[AccountInfo],
        args: &UpdateRecordArgs,
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let setter_account = next_account_info(account_info_iter)?;
        if !setter_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        let repository_account = next_account_info(account_info_iter)?;
        if repository_account.owner != program_id {
            msg!("Derived account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }

        // nft token mint account
        let nft_account = next_account_info(account_info_iter)?;
        
        /* wrong method
        // verify validation of metadata account 
        let nft_metadata_account = next_account_info(account_info_iter)?;
        let metadata_program = spl_token_metadata::id();
        if *nft_metadata_account.owner != metadata_program {
            msg!("Metadata account is not valid. Its owner is not metadata program.");
            return Err(ProgramError::InvalidArgument);
        }
        // read mint account key(nft key) and update authority's (nft owner)
        // from metadata account
        let metadata = Metadata::from_account_info(nft_metadata_account).unwrap();
        // verifying ownership of nft.
        if metadata.mint != *nft_account.key || metadata.update_authority != *setter_account.key {
            msg!("NFT is not owned by signer.");
            return Err(ProgramError::InvalidArgument);
        }
        */

        // verify ownership of nft with owner's associated token account
        // associated token account of hero mint token address
        let associated_token_account = next_account_info(account_info_iter)?;
        let token_account_info = TokenAccount::unpack_from_slice(&associated_token_account.data.borrow())?;
        if token_account_info.owner != *setter_account.key || token_account_info.mint != *nft_account.key {
            msg!("NFT is not owned by signer.");
            return Err(ProgramError::InvalidArgument);
        }

        // get nft listed price from repository account
        let mut nft_record = Self::get_nft_data_from_repository(
            args.hero_id, 
            nft_account.key,
            repository_account.clone(),
            nft_account.clone()
        ).unwrap();

        // update nft last price with listed_price
        nft_record.listed_price = args.new_price;
        nft_record.content_uri = args.content_uri.to_string();
        Self::save_nft_data_to_repository(&nft_record, repository_account.clone())?;

        Ok(())
    }

    /// users can buy seat to present their image
    /// 1. verify ownership of nft(seat) - make sure prev_owner_account is owner of nft
    /// 2. transfer nft from prev_owner to buyer
    /// 3. approve pda to delegate new token account
    /// 4. update last_price of nft record
    /// 5. transfer sol from buyer to prev_owner
    /// 
    fn process_buy_record(
        accounts: &[AccountInfo],
        args: &BuyRecordArgs,
        program_id: &Pubkey
    ) -> ProgramResult {
        msg!("process_buy_record");
        let account_info_iter = &mut accounts.iter();

        let buyer_account = next_account_info(account_info_iter)?;
        if !buyer_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let prev_owner_account = next_account_info(account_info_iter)?;
        
        let repository_account = next_account_info(account_info_iter)?;
        if repository_account.owner != program_id {
            msg!("Derived account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }

        // nft token mint account
        let nft_account = next_account_info(account_info_iter)?;

        // prev_owner's associated token Account to send NFT
        let nft_account_to_send = next_account_info(account_info_iter)?;
        
        // verify ownership of nft with prev_owner's associated token account
        // associated token account of hero mint token address
        let token_account_info = TokenAccount::unpack_from_slice(&nft_account_to_send.data.borrow())?;
        if token_account_info.owner != *prev_owner_account.key || token_account_info.mint != *nft_account.key {
            msg!("NFT is not owned by prev_owner.");
            return Err(ProgramError::InvalidArgument);
        }

        // buyer's token Account to receive NFT
        let nft_account_to_receive = next_account_info(account_info_iter)?;

        let (pda, _nonce) = Pubkey::find_program_address(&[b"hallofheros"], program_id);
        let pda_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        // transfer NFT from 'nft_account_to_send' to 'nft_account_to_receive'
        // pda signed invoke
        let transfer_ix = spl_token::instruction::transfer(
            token_program.key,
            nft_account_to_send.key,
            nft_account_to_receive.key,
            &pda,
            &[],
            1
        )?;
        invoke_signed(
            &transfer_ix,
            &[
                nft_account_to_send.clone(),
                nft_account_to_receive.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"hallofheros"[..], &[_nonce]]]
        )?;
        
        // after receiving NFT, pda needs to get approved for further sale.
        let approve_ix = spl_token::instruction::approve(
            token_program.key,
            nft_account_to_receive.key,
            &pda,
            buyer_account.key,
            &[&buyer_account.key],
            1
        )?;
        invoke(
            &approve_ix,
            &[
                nft_account_to_receive.clone(),
                pda_account.clone(),
                buyer_account.clone(),
                token_program.clone(),
            ],
        )?;
        
        // get nft listed price from repository account
        let mut nft_record = Self::get_nft_data_from_repository(
            args.hero_id, 
            nft_account.key,
            repository_account.clone(),
            nft_account.clone()
        ).unwrap();

        // update nft last price with listed_price
        nft_record.last_price = nft_record.listed_price;
        Self::save_nft_data_to_repository(&nft_record, repository_account.clone())?;

        msg!("before send sol. price={:?}", nft_record.listed_price);
        let system_program_account = next_account_info(account_info_iter)?;

        // transfer sol from buyer to prev_owner
        Self::sol_transfer(
            buyer_account.clone(), 
            prev_owner_account.clone(), 
            system_program_account.clone(),
            nft_record.listed_price
        )?;
        Ok(())
    }

    // transfer sol
    fn sol_transfer<'a>(
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let ix = solana_program::system_instruction::transfer(source.key, destination.key, amount);
        invoke(&ix, &[source, destination, system_program])
    }

    // fetch nft data from repository account with hero_id
    fn get_nft_data_from_repository<'a>(
        hero_id: u8,
        key_nft: &Pubkey,
        repository_account: AccountInfo<'a>,
        nft_account: AccountInfo<'a>,
    ) -> Result<NFTRecord, ProgramError> {
        let start: usize = hero_id as usize * NFT_RECORD_SIZE;
        let end: usize = start + NFT_RECORD_SIZE;

        let nft_record: NFTRecord = NFTRecord::deserialize(&mut &repository_account.data.borrow()[start..end])?;
        
        if nft_record.key_nft != *key_nft || nft_record.key_nft != *nft_account.key {
            msg!("NFT Key dismatch.");
            return Err(HeroError::InvalidNFTKey.into());
        }
        Ok(nft_record)
    }

    // modify nft data to repository
    fn save_nft_data_to_repository<'a>(
        nft_record: &NFTRecord,
        repository_account: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let start: usize = nft_record.hero_id as usize * NFT_RECORD_SIZE;
        let end: usize = start + NFT_RECORD_SIZE;
        nft_record.serialize(&mut &mut repository_account.data.borrow_mut()[start..end])?;
        Ok(())
    }
}
