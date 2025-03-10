use std::{io::Take, marker};

use crate::{error::ApplicationError, state::OrderBook};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_associated_token_account::instruction as associated_token_account_instruction;
use spl_token::{instruction as token_instruction, state::Account as TokenAccount};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TakeOrder {
    id : u64,
    amount: u64,
    price: u64,
}

impl TakeOrder {
    pub fn take_order(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        args: TakeOrder,
    ) -> ProgramResult {

        let [
            user, // user create the order
            taker, // user that accept a particular order
            btc_order_book, // manager config_account
            order_book_admin_pubkey, // manager auth
            token_mint_a, // token_mint that user want to trade for
            token_mint_b, // token_mint that taker want to give in exchange
            user_token_account_b, // user token_account for mint_b
            taker_token_account_a, //taker token_account for mint_a
            taker_token_account_b, // taker token_account for mint_b
            mediator_vault,  //  vault where user token are stored
            token_program_id,
            associated_token_program,
            system_program
            ] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        let btc_order_book_data = OrderBook::try_from_slice(&btc_order_book.data.borrow()[..])?;

        let btc_order_book_seed = &[
            b"btc_order_book",
            order_book_admin_pubkey.key.as_ref(),
            &[btc_order_book_data.bump],
        ];

        let order_book_key = Pubkey::create_program_address(btc_order_book_seed, program_id)?;

        if order_book_key != *btc_order_book.key {
            return Err(ApplicationError::MismatchOrderbookKey.into());
        }

        // create taker token_account for mint_a if needed
        if taker_token_account_a.lamports() == 0 {
            invoke(
                &associated_token_account_instruction::create_associated_token_account(
                    taker.key,
                    taker.key,
                    token_mint_a.key,
                    token_program_id.key,
                ),
                &[
                    taker_token_account_a.clone(),
                    taker.clone(),
                    token_mint_a.clone(),
                    system_program.clone(),
                    token_program_id.clone(),
                    associated_token_program.clone(),
                ],
            )?;
        }

        // create user token_account for mint_b if needed
        if user_token_account_b.lamports() == 0 {
            invoke(
                &associated_token_account_instruction::create_associated_token_account(
                    taker.key,
                    user.key,
                    token_mint_b.key,
                    token_program_id.key,
                ),
                &[
                    token_mint_b.clone(),
                    taker.clone(),
                    user_token_account_b.clone(),
                    user.clone(),
                    system_program.clone(),
                    token_program_id.clone(),
                    associated_token_program.clone(),
                ],
            )?;
        }

        //transfer token from taker to user

        invoke(
            &token_instruction::transfer(
                token_program_id.key,
                taker_token_account_b.key,
                user_token_account_b.key,
                taker.key,
                &[taker.key],
                args.amount * args.price,
            )?,
            &[
                taker.clone(),
                taker_token_account_b.clone(),
                user_token_account_b.clone(),
                token_program_id.clone(),
            ],
        )?;

        //transfer token from mediator_vault to taker

        invoke_signed(
            &token_instruction::transfer(
                token_program_id.key,
                mediator_vault.key,
                taker_token_account_a.key,
                btc_order_book.key,
                &[],
                1,
            )?,
            &[
                token_mint_a.clone(),
                mediator_vault.clone(),
                taker_token_account_a.clone(),
                order_book_admin_pubkey.clone(),
                btc_order_book.clone(),
                taker.clone(),
                token_program_id.clone(),
            ],
            &[btc_order_book_seed],
        )?;

        Ok(())
    }
}
