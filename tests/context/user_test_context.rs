#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use crate::utilities::helper::{
    create_token_account, create_user, get_associated_token_address, get_keypair,
    get_or_create_associated_token_address, get_sysvar_clock, get_token_balance,
    process_instructions,
};
use crate::utilities::kamino::{
    compose_klend_borrow_obligation_liquidity_ix, compose_klend_deposit_obligation_collateral_ix,
    compose_klend_deposit_reserve_liquidity_ix, compose_klend_flash_borrow_ix,
    compose_klend_flash_repay_ix, compose_klend_init_obligation_farms_for_reserve_ix,
    compose_klend_init_obligation_ix, compose_klend_init_user_metadata_ix,
    compose_klend_redeem_reserve_collateral_ix, compose_klend_refresh_obligation_ix,
    compose_klend_refresh_reserve_ix, compose_klend_repay_obligation_liquidity_ix,
    compose_klend_withdraw_obligation_collateral_ix, compose_mock_swap_sol_to_jitosol_ix,
    JITOSOL_MINT, KLEND_PROGRAM_ID, MAIN_MARKET, MAIN_MARKET_AUTHORITY,
    RESERVE_JITOSOL_COLLATERAL_MINT, RESERVE_JITOSOL_COLLATERAL_SUPPLY_VAULT,
    RESERVE_JITOSOL_LIQUIDITY_SUPPLY_VAULT, RESERVE_JITOSOL_STATE, RESERVE_SOL_COLLATERAL_MINT,
    RESERVE_SOL_FARM_STATE, RESERVE_SOL_LIQUIDITY_FEE_VAULT, RESERVE_SOL_LIQUIDITY_MINT,
    RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT, RESERVE_SOL_STATE, RESERVE_USDC_LIQUIDITY_FEE_VAULT,
    RESERVE_USDC_LIQUIDITY_MINT, RESERVE_USDC_LIQUIDITY_SUPPLY_VAULT, RESERVE_USDC_STATE,
};
use solana_program::address_lookup_table;
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::account::Account;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use spl_token;
use std::{cell::RefCell, rc::Rc};

pub struct UserTestContext {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub admin: Keypair,
    pub user: Keypair,
}

impl UserTestContext {
    pub async fn new(context: Rc<RefCell<ProgramTestContext>>) -> UserTestContext {
        let admin = get_keypair("tests/fixtures/admin.json").await;

        let user = create_user(&mut context.borrow_mut()).await;

        UserTestContext {
            context,
            admin,
            user,
        }
    }

    pub async fn new_admin_user(context: Rc<RefCell<ProgramTestContext>>) -> UserTestContext {
        let admin = get_keypair("tests/fixtures/admin.json").await;

        UserTestContext {
            context,
            admin: admin.insecure_clone(),
            user: admin,
        }
    }

    pub async fn get_account(&self, account_pubkey: Pubkey) -> Account {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(account_pubkey)
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn balance(&self) -> u64 {
        self.get_account(self.user.pubkey()).await.lamports
    }

    pub async fn mint_balance(&self, mint: &Pubkey) -> u64 {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let account = get_associated_token_address(&self.user.pubkey(), mint).await;
        let balance = get_token_balance(&mut context.banks_client, account).await;

        balance
    }

    pub async fn assert_mint_balance(&self, mint: Pubkey, expect: u64) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let account = get_associated_token_address(&self.user.pubkey(), &mint).await;
        let balance = get_token_balance(&mut context.banks_client, account).await;

        assert_eq!(balance, expect);
    }

    pub async fn klend_init_user_metadata(&self) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let (user_metadata, _) = Pubkey::find_program_address(
            &[b"user_meta", &self.user.pubkey().to_bytes()],
            &KLEND_PROGRAM_ID,
        );

        let clock = get_sysvar_clock(&mut context.banks_client).await;

        let (_, user_lookup_table) = address_lookup_table::instruction::create_lookup_table(
            self.user.pubkey(),
            self.user.pubkey(),
            clock.slot,
        );

        let instruction = compose_klend_init_user_metadata_ix(
            &self.user.pubkey(),
            &self.user.pubkey(),
            &user_metadata,
            &user_lookup_table,
        );

        process_instructions(context, &self.user, &vec![instruction]).await;
    }

    pub async fn klend_init_obligation(&self, market: &Pubkey, tag: u8, id: u8) -> Pubkey {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let seed_account = Pubkey::default();
        let (obligation, _) = Pubkey::find_program_address(
            &[
                &[tag],
                &[id],
                &self.user.pubkey().to_bytes(),
                &market.to_bytes(),
                &seed_account.to_bytes(),
                &seed_account.to_bytes(),
            ],
            &KLEND_PROGRAM_ID,
        );

        let (user_metadata, _) = Pubkey::find_program_address(
            &[b"user_meta", &self.user.pubkey().to_bytes()],
            &KLEND_PROGRAM_ID,
        );

        let instruction = compose_klend_init_obligation_ix(
            &self.user.pubkey(),
            &self.user.pubkey(),
            &obligation,
            market,
            &seed_account,
            &seed_account,
            &user_metadata,
            tag,
            id,
        );

        process_instructions(context, &self.user, &vec![instruction]).await;

        obligation
    }

    pub async fn klend_refresh_reserve(&self, reserve: &Pubkey) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let instruction = compose_klend_refresh_reserve_ix(reserve, &MAIN_MARKET);

        process_instructions(context, &self.user, &vec![instruction]).await;
    }

    pub async fn klend_refresh_obligation(&self, obligation: &Pubkey, reserves: &Vec<Pubkey>) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let instruction = compose_klend_refresh_obligation_ix(obligation, &MAIN_MARKET, reserves);

        process_instructions(context, &self.user, &vec![instruction]).await;
    }

    pub async fn klend_deposit_reserve_jitosol_liquidity(&self, liquidity_amount: u64) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let (lending_market_authority, _) =
            Pubkey::find_program_address(&[b"lma", &MAIN_MARKET.to_bytes()], &KLEND_PROGRAM_ID);

        println!("Main market authority: {}", lending_market_authority);

        let user_source_liquidity = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &JITOSOL_MINT,
        )
        .await;
        let user_destination_collateral = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &RESERVE_JITOSOL_COLLATERAL_MINT,
        )
        .await;

        let instruction = compose_klend_deposit_reserve_liquidity_ix(
            &self.user.pubkey(),
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
            &lending_market_authority,
            &JITOSOL_MINT,
            &RESERVE_JITOSOL_LIQUIDITY_SUPPLY_VAULT,
            &RESERVE_JITOSOL_COLLATERAL_MINT,
            &user_source_liquidity,
            &user_destination_collateral,
            liquidity_amount,
        );

        process_instructions(context, &self.user, &vec![instruction]).await;
    }

    pub async fn klend_redeem_reserve_jitosol_collateral(&self) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let (lending_market_authority, _) =
            Pubkey::find_program_address(&[b"lma", &MAIN_MARKET.to_bytes()], &KLEND_PROGRAM_ID);

        let user_destination_liquidity = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &JITOSOL_MINT,
        )
        .await;

        let user_source_collateral = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &RESERVE_JITOSOL_COLLATERAL_MINT,
        )
        .await;

        let collateral_amount =
            get_token_balance(&mut context.banks_client, user_source_collateral).await;

        let instruction = compose_klend_redeem_reserve_collateral_ix(
            &self.user.pubkey(),
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
            &lending_market_authority,
            &JITOSOL_MINT,
            &RESERVE_JITOSOL_LIQUIDITY_SUPPLY_VAULT,
            &RESERVE_JITOSOL_COLLATERAL_MINT,
            &user_source_collateral,
            &user_destination_liquidity,
            collateral_amount,
        );

        process_instructions(context, &self.user, &vec![instruction]).await;

        let jitosol_balance =
            get_token_balance(&mut context.banks_client, user_destination_liquidity).await;

        println!("redeem jitosol balance: {}", jitosol_balance);
    }

    pub async fn klend_deposit_reserve_sol_liquidity(&self, liquidity_amount: u64) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let (lending_market_authority, _) =
            Pubkey::find_program_address(&[b"lma", &MAIN_MARKET.to_bytes()], &KLEND_PROGRAM_ID);

        println!("Main market authority: {}", lending_market_authority);

        let user_wsol_acc = Keypair::new();

        create_token_account(
            context,
            &self.user,
            &user_wsol_acc,
            &spl_token::native_mint::id(),
            &self.user.pubkey(),
            liquidity_amount,
        )
        .await
        .unwrap();

        let user_destination_collateral = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &RESERVE_SOL_COLLATERAL_MINT,
        )
        .await;

        let mut instructions: Vec<Instruction> = vec![];

        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_SOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_deposit_reserve_liquidity_ix(
            &self.user.pubkey(),
            &RESERVE_SOL_STATE,
            &MAIN_MARKET,
            &lending_market_authority,
            &RESERVE_SOL_LIQUIDITY_MINT,
            &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
            &RESERVE_SOL_COLLATERAL_MINT,
            &user_wsol_acc.pubkey(),
            &user_destination_collateral,
            liquidity_amount,
        ));

        let close_wsol_account_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &user_wsol_acc.pubkey(),
            &self.user.pubkey(),
            &self.user.pubkey(),
            &[&self.user.pubkey()],
        )
        .unwrap();

        instructions.push(close_wsol_account_ix);

        process_instructions(context, &self.user, &instructions).await;
    }

    pub async fn klend_deposit_obligation_collateral(&self, obligation: &Pubkey) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let user_source_collateral =
            get_associated_token_address(&self.user.pubkey(), &RESERVE_JITOSOL_COLLATERAL_MINT)
                .await;
        let collateral_amount =
            get_token_balance(&mut context.banks_client, user_source_collateral).await;

        println!(
            "deposit obligation collateral amount: {}",
            collateral_amount
        );

        let mut instructions: Vec<Instruction> = vec![];
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![],
        ));

        instructions.push(compose_klend_deposit_obligation_collateral_ix(
            &self.user.pubkey(),
            obligation,
            &MAIN_MARKET,
            &RESERVE_JITOSOL_STATE,
            &RESERVE_JITOSOL_COLLATERAL_SUPPLY_VAULT,
            &user_source_collateral,
            collateral_amount,
        ));

        process_instructions(context, &self.user, &instructions).await;

        let collateral_amount =
            get_token_balance(&mut context.banks_client, user_source_collateral).await;
        assert_eq!(collateral_amount, 0);
    }

    pub async fn klend_withdraw_obligation_collateral(&self, obligation: &Pubkey) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let user_destination_collateral =
            get_associated_token_address(&self.user.pubkey(), &RESERVE_JITOSOL_COLLATERAL_MINT)
                .await;

        let mut instructions: Vec<Instruction> = vec![];
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_SOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![RESERVE_JITOSOL_STATE, RESERVE_SOL_STATE],
        ));

        instructions.push(compose_klend_withdraw_obligation_collateral_ix(
            &self.user.pubkey(),
            obligation,
            &MAIN_MARKET,
            &MAIN_MARKET_AUTHORITY,
            &RESERVE_JITOSOL_STATE,
            &RESERVE_JITOSOL_COLLATERAL_SUPPLY_VAULT,
            &user_destination_collateral,
            u64::MAX,
        ));

        process_instructions(context, &self.user, &instructions).await;

        let collateral_amount =
            get_token_balance(&mut context.banks_client, user_destination_collateral).await;
        println!(
            "withdraw obligation collateral amount: {}",
            collateral_amount
        );
    }

    pub async fn klend_borrow_obligation_liquidity(
        &self,
        obligation: &Pubkey,
        mint: &str,
        liquidity_amount: u64,
    ) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mut instructions: Vec<Instruction> = vec![];
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        match mint {
            "SOL" => {
                instructions.push(compose_klend_refresh_reserve_ix(
                    &RESERVE_SOL_STATE,
                    &MAIN_MARKET,
                ));
            }
            "USDC" => {
                instructions.push(compose_klend_refresh_reserve_ix(
                    &RESERVE_USDC_STATE,
                    &MAIN_MARKET,
                ));
            }
            _ => panic!("not support"),
        }

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![RESERVE_JITOSOL_STATE],
        ));

        match mint {
            "SOL" => {
                let user_destination_liquidity = get_or_create_associated_token_address(
                    context,
                    &self.user,
                    &self.user.pubkey(),
                    &RESERVE_SOL_LIQUIDITY_MINT,
                )
                .await;

                instructions.push(compose_klend_borrow_obligation_liquidity_ix(
                    &self.user.pubkey(),
                    obligation,
                    &MAIN_MARKET,
                    &MAIN_MARKET_AUTHORITY,
                    &RESERVE_SOL_STATE,
                    &RESERVE_SOL_LIQUIDITY_MINT,
                    &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
                    &RESERVE_SOL_LIQUIDITY_FEE_VAULT,
                    &user_destination_liquidity,
                    liquidity_amount,
                ));

                instructions.push(
                    spl_token::instruction::close_account(
                        &spl_token::id(),
                        &user_destination_liquidity,
                        &self.user.pubkey(),
                        &self.user.pubkey(),
                        &[&self.user.pubkey()],
                    )
                    .unwrap(),
                );
            }
            "USDC" => {
                let user_destination_liquidity = get_or_create_associated_token_address(
                    context,
                    &self.user,
                    &self.user.pubkey(),
                    &RESERVE_USDC_LIQUIDITY_MINT,
                )
                .await;

                instructions.push(compose_klend_borrow_obligation_liquidity_ix(
                    &self.user.pubkey(),
                    obligation,
                    &MAIN_MARKET,
                    &MAIN_MARKET_AUTHORITY,
                    &RESERVE_USDC_STATE,
                    &RESERVE_USDC_LIQUIDITY_MINT,
                    &RESERVE_USDC_LIQUIDITY_SUPPLY_VAULT,
                    &RESERVE_USDC_LIQUIDITY_FEE_VAULT,
                    &user_destination_liquidity,
                    liquidity_amount,
                ));
            }
            _ => panic!("not support"),
        }

        process_instructions(context, &self.user, &instructions).await;
    }

    pub async fn klend_repay_obligation_liquidity(
        &self,
        obligation: &Pubkey,
        mint: &str,
        liquidity_amount: u64,
    ) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mut instructions: Vec<Instruction> = vec![];
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        match mint {
            "SOL" => {
                instructions.push(compose_klend_refresh_reserve_ix(
                    &RESERVE_SOL_STATE,
                    &MAIN_MARKET,
                ));
            }
            "USDC" => {
                instructions.push(compose_klend_refresh_reserve_ix(
                    &RESERVE_USDC_STATE,
                    &MAIN_MARKET,
                ));
            }
            _ => panic!("not support"),
        }

        let additional_reserve = match mint {
            "SOL" => RESERVE_SOL_STATE,
            "USDC" => RESERVE_USDC_STATE,
            _ => panic!("not support"),
        };

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![RESERVE_JITOSOL_STATE, additional_reserve],
        ));

        match mint {
            "SOL" => {
                let user_wsol_acc = Keypair::new();

                create_token_account(
                    context,
                    &self.user,
                    &user_wsol_acc,
                    &spl_token::native_mint::id(),
                    &self.user.pubkey(),
                    liquidity_amount,
                )
                .await
                .unwrap();

                instructions.push(compose_klend_repay_obligation_liquidity_ix(
                    &self.user.pubkey(),
                    obligation,
                    &MAIN_MARKET,
                    &RESERVE_SOL_STATE,
                    &RESERVE_SOL_LIQUIDITY_MINT,
                    &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
                    &user_wsol_acc.pubkey(),
                    liquidity_amount,
                ));

                instructions.push(
                    spl_token::instruction::close_account(
                        &spl_token::id(),
                        &user_wsol_acc.pubkey(),
                        &self.user.pubkey(),
                        &self.user.pubkey(),
                        &[&self.user.pubkey()],
                    )
                    .unwrap(),
                );
            }
            "USDC" => {
                let user_source_liquidity = get_or_create_associated_token_address(
                    context,
                    &self.user,
                    &self.user.pubkey(),
                    &RESERVE_USDC_LIQUIDITY_MINT,
                )
                .await;

                instructions.push(compose_klend_repay_obligation_liquidity_ix(
                    &self.user.pubkey(),
                    obligation,
                    &MAIN_MARKET,
                    &RESERVE_USDC_STATE,
                    &RESERVE_USDC_LIQUIDITY_MINT,
                    &RESERVE_USDC_LIQUIDITY_SUPPLY_VAULT,
                    &user_source_liquidity,
                    liquidity_amount,
                ));
            }
            _ => panic!("not support"),
        }

        process_instructions(context, &self.user, &instructions).await;
    }

    pub async fn mock_swap_sol_to_jitosol(&self, amount: u64) {
        assert_eq!(self.user.pubkey(), self.admin.pubkey());
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let user_wsol_account = Keypair::new();
        create_token_account(
            context,
            &self.user,
            &user_wsol_account,
            &spl_token::native_mint::id(),
            &self.user.pubkey(),
            amount,
        )
        .await
        .unwrap();

        let temp_destination_account = Keypair::new();
        create_token_account(
            context,
            &self.user,
            &temp_destination_account,
            &spl_token::native_mint::id(),
            &self.user.pubkey(),
            0,
        )
        .await
        .unwrap();

        let user_jitosol_account = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &JITOSOL_MINT,
        )
        .await;

        let instructions = compose_mock_swap_sol_to_jitosol_ix(
            &self.user.pubkey(),
            &user_wsol_account.pubkey(),
            &temp_destination_account.pubkey(),
            &user_jitosol_account,
            amount,
            12000,
        );

        process_instructions(context, &self.user, &instructions).await;
    }

    pub async fn leverage_borrow(&self, obligation: &Pubkey) {
        assert_eq!(self.user.pubkey(), self.admin.pubkey()); // must be mint authority of jitosol for ease of test
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let mut instructions: Vec<Instruction> = vec![];

        // 1. Flash borrow 20 sol
        let user_destination_liquidity = Keypair::new();
        create_token_account(
            context,
            &self.user,
            &user_destination_liquidity,
            &spl_token::native_mint::id(),
            &self.user.pubkey(),
            0,
        )
        .await
        .unwrap();

        instructions.push(compose_klend_flash_borrow_ix(
            &self.user.pubkey(),
            &MAIN_MARKET,
            &MAIN_MARKET_AUTHORITY,
            &RESERVE_SOL_STATE,
            &RESERVE_SOL_LIQUIDITY_MINT,
            &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
            &user_destination_liquidity.pubkey(),
            &RESERVE_SOL_LIQUIDITY_FEE_VAULT,
            20_000_000_000,
        ));

        // 2. Swap sol to jitosol
        let temp_destination_account = Keypair::new();
        create_token_account(
            context,
            &self.user,
            &temp_destination_account,
            &spl_token::native_mint::id(),
            &self.user.pubkey(),
            0,
        )
        .await
        .unwrap();

        let user_jitosol_account = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &JITOSOL_MINT,
        )
        .await;

        instructions.extend_from_slice(&compose_mock_swap_sol_to_jitosol_ix(
            &self.user.pubkey(),
            &user_destination_liquidity.pubkey(),
            &temp_destination_account.pubkey(),
            &user_jitosol_account,
            20_000_000_000,
            12000,
        ));

        // 3. Deposit jitosol to reserve
        let user_jitosol_collateral_account = get_or_create_associated_token_address(
            context,
            &self.user,
            &self.user.pubkey(),
            &RESERVE_JITOSOL_COLLATERAL_MINT,
        )
        .await;

        instructions.push(compose_klend_deposit_reserve_liquidity_ix(
            &self.user.pubkey(),
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
            &MAIN_MARKET_AUTHORITY,
            &JITOSOL_MINT,
            &RESERVE_JITOSOL_LIQUIDITY_SUPPLY_VAULT,
            &RESERVE_JITOSOL_COLLATERAL_MINT,
            &user_jitosol_account,
            &user_jitosol_collateral_account,
            51_600_000_000,
        ));

        // 4. Deposit obligation collateral
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![],
        ));

        instructions.push(compose_klend_deposit_obligation_collateral_ix(
            &self.user.pubkey(),
            obligation,
            &MAIN_MARKET,
            &RESERVE_JITOSOL_STATE,
            &RESERVE_JITOSOL_COLLATERAL_SUPPLY_VAULT,
            &user_jitosol_collateral_account,
            51_600_000_000 * 10000 / 10100,
        ));

        // 5. Borrow obligation liquidity: 20 sol
        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_JITOSOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_reserve_ix(
            &RESERVE_SOL_STATE,
            &MAIN_MARKET,
        ));

        instructions.push(compose_klend_refresh_obligation_ix(
            obligation,
            &MAIN_MARKET,
            &vec![RESERVE_JITOSOL_STATE],
        ));

        instructions.push(compose_klend_borrow_obligation_liquidity_ix(
            &self.user.pubkey(),
            obligation,
            &MAIN_MARKET,
            &MAIN_MARKET_AUTHORITY,
            &RESERVE_SOL_STATE,
            &RESERVE_SOL_LIQUIDITY_MINT,
            &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
            &RESERVE_SOL_LIQUIDITY_FEE_VAULT,
            &user_destination_liquidity.pubkey(),
            20_000_000_000,
        ));

        // 6. Flash repay 20 sol
        instructions.push(compose_klend_flash_repay_ix(
            &self.user.pubkey(),
            &MAIN_MARKET,
            &MAIN_MARKET_AUTHORITY,
            &RESERVE_SOL_STATE,
            &RESERVE_SOL_LIQUIDITY_MINT,
            &RESERVE_SOL_LIQUIDITY_SUPPLY_VAULT,
            &user_destination_liquidity.pubkey(), // Must be the same one of borrow ix
            &RESERVE_SOL_LIQUIDITY_FEE_VAULT,
            20_000_000_000,
            0,
        ));

        process_instructions(context, &self.user, &instructions).await;
    }

    pub async fn klend_init_obligation_farms_for_reserve(
        &self,
        obligation: &Pubkey,
        reserve_name: &str,
    ) {
        let context: &mut ProgramTestContext = &mut self.context.borrow_mut();

        let (reserve, reserve_farm_state) = match reserve_name {
            "SOL" => (&RESERVE_SOL_STATE, &RESERVE_SOL_FARM_STATE),
            // "USDC" => (&RESERVE_USDC_STATE, &RESERVE_USDC_FARM_STATE),
            // "JITOSOL" => (&RESERVE_JITOSOL_STATE, &RESERVE_JITOSOL_FARM_STATE),
            _ => panic!("not support"),
        };

        let (obligation_farm, _) = Pubkey::find_program_address(
            &[
                b"user",
                &reserve_farm_state.to_bytes(),
                &obligation.to_bytes(),
            ],
            &KLEND_PROGRAM_ID,
        );

        let instruction = compose_klend_init_obligation_farms_for_reserve_ix(
            &self.user.pubkey(),
            &self.user.pubkey(),
            obligation,
            &MAIN_MARKET_AUTHORITY,
            reserve,
            reserve_farm_state,
            &obligation_farm,
            &MAIN_MARKET,
            0,
        );

        process_instructions(context, &self.user, &vec![instruction]).await;
    }
}
