#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::state::AccountKind;
use bridge_utils::types::{EverAddress, UInt256};
use native_proxy::deposit_ix;
use solana_program::rent::Rent;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::AccountState;

use native_proxy::*;

#[tokio::test]
async fn test_deposit() {
    let mut program_test = ProgramTest::new("native_proxy", id(), processor!(Processor::process));

    program_test.add_program(
        "token_proxy",
        token_proxy::id(),
        processor!(token_proxy::Processor::process),
    );

    // Setup environment

    // Add Sender Account
    let sender = Keypair::new();

    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 1_000_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = token_proxy::get_settings_address();

    let settings_account_data = token_proxy::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; token_proxy::Settings::LEN];
    token_proxy::Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(token_proxy::Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add MultiVault Account
    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());

    let multivault_address = token_proxy::get_multivault_address();

    let multivault_account_data = token_proxy::MultiVault {
        is_initialized: true,
        account_kind: AccountKind::MultiVault(multivault_nonce),
    };

    let mut multivault_packed = vec![0; token_proxy::MultiVault::LEN];
    token_proxy::MultiVault::pack(multivault_account_data, &mut multivault_packed).unwrap();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: Rent::default().minimum_balance(token_proxy::MultiVault::LEN),
            data: multivault_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Vault Account
    let mint_address = spl_token::native_mint::id();

    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = token_proxy::get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut vault_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(vault_account_data, &mut vault_packed).unwrap();
    program_test.add_account(
        vault_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: vault_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "wSOL".to_string();
    let name = "Wrapped SOL".to_string();
    let deposit_limit = 10_000_000_000;
    let withdrawal_limit = 10_000_000_000;
    let withdrawal_daily_limit = 10_000_000_000;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = token_proxy::get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = token_proxy::TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: token_proxy::TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name: name.clone(),
        symbol: symbol.clone(),
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_supply: Default::default(),
        fee_deposit_info: Default::default(),
        fee_withdrawal_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; token_proxy::TokenSettings::LEN];
    token_proxy::TokenSettings::pack(token_settings_account_data, &mut token_settings_packed)
        .unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(token_proxy::TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 1_000_000_000;
    let value = 0;
    let payload: Vec<u8> = vec![];
    let expected_evers = UInt256::default();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_ix(
            funder.pubkey(),
            sender.pubkey(),
            deposit_seed,
            amount,
            recipient,
            value,
            expected_evers,
            payload.clone(),
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Vault Balance
    let vault_address = token_proxy::get_vault_address(&mint_address);

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");
    assert_eq!(vault_data.amount, amount);

    // Check Sender Valance
    let sender_info = banks_client
        .get_account(spl_associated_token_account::get_associated_token_address(
            &sender.pubkey(),
            &mint_address,
        ))
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 0);

    // Check Deposit Account
    let deposit_address = token_proxy::get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = token_proxy::DepositMultiTokenSol::unpack(deposit_info.data())
        .expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[br"deposit", &deposit_seed.to_le_bytes()],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.base_token, mint_address);
    assert_eq!(deposit_data.event.data.name, name);
    assert_eq!(deposit_data.event.data.symbol, symbol);
    assert_eq!(
        deposit_data.event.data.decimals,
        spl_token::native_mint::DECIMALS
    );
    assert_eq!(deposit_data.event.data.value, value);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

    let token_settings_address = token_proxy::get_token_settings_sol_address(&mint_address);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data = token_proxy::TokenSettings::unpack(token_settings_info.data())
        .expect("deposit token unpack");

    let fee_info = &token_settings_data.fee_deposit_info;
    let fee = 1.max(
        (amount)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount - fee;

    assert_eq!(deposit_data.event.data.amount, transfer_amount as u128);

    // Check Deposit Account to unpack
    let raw_deposit_data =
        token_proxy::Deposit::unpack_from_slice(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(
        raw_deposit_data.event,
        deposit_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        raw_deposit_data.meta,
        deposit_data.meta.data.try_to_vec().unwrap()
    );
}
