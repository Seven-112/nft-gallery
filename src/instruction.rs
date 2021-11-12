use solana_program::{
    program_error::ProgramError,
    msg
};

use borsh::{BorshDeserialize};

use crate::error::HeroError::InvalidInstruction;

use crate::processor::{
    AddRecordArgs, UpdateRecordArgs, BuyRecordArgs
};

pub enum HeroInstruction {

    /// Add Heros into Repository Account
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person adding the hero
    /// 1. `[writable]` Our repository account should be created prior to this instruction. It will hold all infos about our heros.
    /// 2. `[]` The associated_token_account of nft mint token account
    /// 3. `[]` PDA of this repository program to get approved from ATokenAccount
    /// 4. `[]` Token Program Account
    AddRecord(AddRecordArgs),

    /// Set Hero price
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the setter & signer
    /// 1. `[writable]` Our repository account which saves all onchain data
    /// 2. `[]` The NFT mint token account of which price will be changed
    /// 3. `[]` The associated_token_account of nft mint token account
    
    UpdateRecord(UpdateRecordArgs),

    /// Buy Hero
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer, writable]` The account of the person buys hero
    /// 1. `[writable]` Previous owner of nft
    /// 2. `[writable]` Repository account
    /// 3. `[]` The NFT mint token account of which price will be changed
    /// 4. `[]` The NFT token account from which send token
    /// 5. `[]` The NFT token account to which receive token
    /// 6. `[]` PDA of this repository program to get approved from ATokenAccount
    /// 7. `[]` Token Program Account
    /// 8. `[]` System Program Account
    
    BuyRecord(BuyRecordArgs),
}

impl HeroInstruction{
    
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        
        msg!("instruction unpack: tag is {}", tag);

        Ok(match tag {
            0 => {
                Self::AddRecord(Self::unpack_add_record_args(rest)?)
            },
            1 => {
                Self::UpdateRecord(Self::unpack_update_record_args(rest)?)
            },
            2 => {
                Self::BuyRecord(Self::unpack_buy_record_args(rest)?)
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_add_record_args(input: &[u8]) -> Result<AddRecordArgs, ProgramError> {
        let args = AddRecordArgs::try_from_slice(input)?;
        Ok(args)
    }

    fn unpack_update_record_args(input: &[u8]) -> Result<UpdateRecordArgs, ProgramError> {
        let args = UpdateRecordArgs::try_from_slice(input)?;
        Ok(args)
    }

    fn unpack_buy_record_args(input: &[u8]) -> Result<BuyRecordArgs, ProgramError> {
        let args = BuyRecordArgs::try_from_slice(input)?;
        Ok(args)
    }
}
