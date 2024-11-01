#![cfg(test)]

mod context;
mod utilities;

use context::{MintType, RateXTestContext};
use solana_program_test::*;
use utilities::kamino::{
    dump_reserve, EXAMPLE_OBLIGATION, JITOSOL_MINT, MAIN_MARKET, RESERVE_JITOSOL_STATE,
    RESERVE_SOL_STATE, RESERVE_USDC_STATE,
};

#[tokio::test]
async fn test_kamino() {
    let rtc = RateXTestContext::new().await;
    let alice = &rtc.users[1];

    rtc.set_sysvar_clock(1730163565).await; // KAMINO_SCOPE_PRICES

    dump_reserve(&RESERVE_SOL_STATE);
    // dump_reserve(&RESERVE_USDC_STATE);

    alice.klend_init_user_metadata().await;
    let obligation = alice.klend_init_obligation(&MAIN_MARKET, 0, 0).await;
    alice.klend_refresh_reserve(&RESERVE_JITOSOL_STATE).await;
    alice.klend_refresh_obligation(&obligation, &vec![]).await;

    rtc.mint_token_by_type(alice, 50_000_000_000, MintType::JitoSol)
        .await;
    alice
        .assert_mint_balance(JITOSOL_MINT, 50_000_000_000)
        .await;

    alice
        .klend_deposit_reserve_jitosol_liquidity(50_000_000_000)
        .await;

    alice.klend_deposit_obligation_collateral(&obligation).await;

    println!(
        "user sol balance before borrowing: {}",
        alice.balance().await
    );
    alice
        .klend_borrow_obligation_liquidity(&obligation, "SOL", 20_000_000_000)
        .await;
    println!(
        "user sol balance after borrowing: {}",
        alice.balance().await
    );

    alice
        .klend_repay_obligation_liquidity(&obligation, "SOL", 20_000_000_000)
        .await;
    println!("user sol balance after repaying: {}", alice.balance().await);

    alice
        .klend_withdraw_obligation_collateral(&obligation)
        .await;

    alice.klend_redeem_reserve_jitosol_collateral().await;
}

#[tokio::test]
async fn test_mock_swap_sol_to_jitosol() {
    let rtc = RateXTestContext::new().await;
    let admin = &rtc.users[0];

    rtc.mint_token_by_type(admin, 8_000_000_000_000, MintType::Sol)
        .await;

    println!("admin sol balance before swap: {}", admin.balance().await);
    admin.mock_swap_sol_to_jitosol(8_000_000_000_000).await;
    println!("admin sol balance after swap: {}", admin.balance().await);

    admin.assert_mint_balance(JITOSOL_MINT, 6666666666666).await;
}

#[tokio::test]
async fn test_leverage_borrow() {
    let rtc = RateXTestContext::new().await;
    let admin = &rtc.users[0];

    rtc.set_sysvar_clock(1730163565).await;
    admin.klend_init_user_metadata().await;
    let obligation = admin.klend_init_obligation(&MAIN_MARKET, 0, 0).await;

    rtc.mint_token_by_type(admin, 35_000_000_000, MintType::JitoSol)
        .await;

    // 51.6 jitosol， borrow 20 sol
    admin.enter_leverage_borrow(&obligation).await;

    admin.leave_leverage_borrow(&obligation).await;
}

#[tokio::test]
async fn test_obligation() {
    let rtc = RateXTestContext::new().await;
    let admin = &rtc.users[0];

    admin.dump_obligation(&EXAMPLE_OBLIGATION).await;
    dump_reserve(&RESERVE_USDC_STATE);
}
