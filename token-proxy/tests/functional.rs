#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::{EverAddress, UInt256, Vote, RELAY_REPARATION};

use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::hash::hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::rent::Rent;
use solana_program::{bpf_loader_upgradeable, program_option, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::AccountState;

use token_proxy::*;

#[tokio::test]
async fn test_init_settings() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let initializer = Keypair::new();

    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(initializer.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[initialize_settings_ix(
            funder.pubkey(),
            initializer.pubkey(),
            guardian,
            manager,
            withdrawal_manager,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &initializer], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Settings Account
    let settings_address = get_settings_address();
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.emergency, false);
    assert_eq!(settings_data.guardian, guardian);
    assert_eq!(settings_data.manager, manager);
    assert_eq!(settings_data.withdrawal_manager, withdrawal_manager);

    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());
    assert_eq!(
        settings_data.account_kind,
        AccountKind::Settings(settings_nonce, programdata_nonce)
    );

    // Check MultiVault Account
    let multivault_address = get_multivault_address();
    let multivault_info = banks_client
        .get_account(multivault_address)
        .await
        .expect("get_account")
        .expect("account");

    let multivault_data = MultiVault::unpack(multivault_info.data()).expect("multivault unpack");
    assert_eq!(multivault_data.is_initialized, true);

    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());
    assert_eq!(
        multivault_data.account_kind,
        AccountKind::MultiVault(multivault_nonce)
    );
}

#[tokio::test]
async fn test_deposit_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = 9;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let fee_info = token_settings_account_data.fee_info.clone();

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add MultiVault  Account
    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());

    let multivault_address = get_multivault_address();

    let multivault_account_data = MultiVault {
        is_initialized: true,
        account_kind: AccountKind::MultiVault(multivault_nonce),
    };

    let mut multivault_packed = vec![0; MultiVault::LEN];
    MultiVault::pack(multivault_account_data, &mut multivault_packed).unwrap();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: Rent::default().minimum_balance(MultiVault::LEN),
            data: multivault_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Account
    let sender = Keypair::new();

    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint_address);

    let sender_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: sender.pubkey(),
        amount: 100,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut sender_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(sender_account_data, &mut sender_packed).unwrap();
    program_test.add_account(
        sender_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: sender_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let value = 1000;
    let payload: Vec<u8> = vec![];
    let expected_evers = UInt256::default();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_ever_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            &token,
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

    // Check Mint Supply
    let mint_info = banks_client
        .get_account(mint_address)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, 100 - amount);

    // Check Sender Balance
    let sender_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 100 - amount);

    // Check Deposit Account
    let deposit_address = get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenEver::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[br"deposit", &deposit_seed.to_le_bytes()],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.token, token);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

    let fee = 1.max(
        (amount)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount - fee;

    assert_eq!(deposit_data.event.data.amount, transfer_amount as u128);
}

#[tokio::test]
async fn test_deposit_ever_for_18_decimals() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = 18;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals: spl_token::native_mint::DECIMALS,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = 10_000_000;
    let withdrawal_limit = 10_000;
    let withdrawal_daily_limit = 1_000;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let fee_info = token_settings_account_data.fee_info.clone();

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add MultiVault  Account
    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());

    let multivault_address = get_multivault_address();

    let multivault_account_data = MultiVault {
        is_initialized: true,
        account_kind: AccountKind::MultiVault(multivault_nonce),
    };

    let mut multivault_packed = vec![0; MultiVault::LEN];
    MultiVault::pack(multivault_account_data, &mut multivault_packed).unwrap();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: Rent::default().minimum_balance(MultiVault::LEN),
            data: multivault_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Account
    let sender = Keypair::new();

    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint_address);

    let sender_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: sender.pubkey(),
        amount: 100,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut sender_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(sender_account_data, &mut sender_packed).unwrap();
    program_test.add_account(
        sender_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: sender_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let value = 1000;
    let payload: Vec<u8> = vec![];
    let expected_evers = UInt256::default();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_ever_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            &token,
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

    // Check Mint Supply
    let mint_info = banks_client
        .get_account(mint_address)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, 100 - amount);

    // Check Sender Valance
    let sender_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 100 - amount);

    // Check Deposit Account
    let deposit_address = get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenEver::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[br"deposit", &deposit_seed.to_le_bytes()],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.token, token);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

    let fee = 1.max(
        (amount)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount - fee;

    assert_eq!(
        deposit_data.event.data.amount,
        (transfer_amount * 1_000_000_000) as u128
    );

    // Check Deposit Account to unpack
    let raw_deposit_data =
        Deposit::unpack_from_slice(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(
        raw_deposit_data.event,
        deposit_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        raw_deposit_data.meta,
        deposit_data.meta.data.try_to_vec().unwrap()
    );
}

#[tokio::test]
async fn test_deposit_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let mint = Pubkey::new_unique();

    let decimals = spl_token::native_mint::DECIMALS;

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add MultiVault Account
    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());

    let multivault_address = get_multivault_address();

    let multivault_account_data = MultiVault {
        is_initialized: true,
        account_kind: AccountKind::MultiVault(multivault_nonce),
    };

    let mut multivault_packed = vec![0; MultiVault::LEN];
    MultiVault::pack(multivault_account_data, &mut multivault_packed).unwrap();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: Rent::default().minimum_balance(MultiVault::LEN),
            data: multivault_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Account
    let sender = Keypair::new();

    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint);

    let sender_account_data = spl_token::state::Account {
        mint,
        owner: sender.pubkey(),
        amount: 100,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut sender_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(sender_account_data, &mut sender_packed).unwrap();
    program_test.add_account(
        sender_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: sender_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let value = 1000;
    let payload: Vec<u8> = vec![];
    let expected_evers = UInt256::default();
    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_sol_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            mint,
            deposit_seed,
            name.clone(),
            symbol.clone(),
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
    let vault_address = get_vault_address(&mint);

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");
    assert_eq!(vault_data.amount, amount);

    // Check Sender Valance
    let sender_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 100 - amount);

    // Check Token Settings Account
    let token_settings_address = get_token_settings_sol_address(&mint);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("deposit token unpack");

    assert_eq!(token_settings_data.is_initialized, true);
    assert_eq!(token_settings_data.withdrawal_epoch, 0);
    assert_eq!(token_settings_data.deposit_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_amount, 0);
    assert_eq!(token_settings_data.emergency, false);

    assert_eq!(
        token_settings_data.kind,
        TokenKind::Solana {
            mint,
            vault: vault_address
        }
    );

    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], &token_proxy::id());
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint.to_bytes()], &token_proxy::id());

    assert_eq!(
        token_settings_data.account_kind,
        AccountKind::TokenSettings(token_settings_nonce, vault_nonce)
    );

    // Check Deposit Account
    let deposit_address = get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenSol::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[br"deposit", &deposit_seed.to_le_bytes()],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.base_token, mint);
    assert_eq!(deposit_data.event.data.name, name);
    assert_eq!(deposit_data.event.data.symbol, symbol);
    assert_eq!(deposit_data.event.data.decimals, decimals);
    assert_eq!(deposit_data.event.data.value, value);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

    let fee_info = &token_settings_data.fee_info;
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
        Deposit::unpack_from_slice(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(
        raw_deposit_data.event,
        deposit_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        raw_deposit_data.meta,
        deposit_data.meta.data.try_to_vec().unwrap()
    );
}

#[tokio::test]
async fn test_withdraw_ever_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Round Account
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.clone(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload: Vec<u8> = vec![];
    let attached_amount = 0;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_ever_request_ix(
            funder.pubkey(),
            author.pubkey(),
            round_number,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            token,
            name.clone(),
            symbol.clone(),
            decimals,
            recipient,
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient,
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.token, token);
    assert_eq!(withdrawal_data.event.data.name, name);
    assert_eq!(withdrawal_data.event.data.symbol, symbol);
    assert_eq!(withdrawal_data.event.data.decimals, decimals);
    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.event.data.recipient, recipient);

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );
    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, None)
    );

    // Check Proposal Account to unpack
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );
}

#[tokio::test]
async fn test_withdraw_ever_request_with_fake_payload() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Round Account
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.clone(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload = bincode::serialize(&vec![solana_program::system_instruction::allocate(
        &Pubkey::new_unique(),
        0,
    )])
    .unwrap();

    let attached_amount = 10;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_ever_request_ix(
            funder.pubkey(),
            author.pubkey(),
            round_number,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            token,
            name.clone(),
            symbol.clone(),
            decimals,
            recipient,
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient,
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.token, token);
    assert_eq!(withdrawal_data.event.data.name, name);
    assert_eq!(withdrawal_data.event.data.symbol, symbol);
    assert_eq!(withdrawal_data.event.data.decimals, decimals);
    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.event.data.recipient, recipient);

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let mint = get_mint_address(&token);

    let (_, proxy_nonce) = Pubkey::find_program_address(
        &[br"proxy", &mint.to_bytes(), &recipient.to_bytes()],
        &token_proxy::id(),
    );

    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, Some(proxy_nonce))
    );

    // Check Proposal Account to unpack
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );

    // Check Proxy Account
    let proxy_address = get_proxy_address(&mint, &recipient);

    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    assert_eq!(proxy_info.data.len(), spl_token::state::Account::LEN);
}

#[tokio::test]
async fn test_withdraw_sol_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Round Account
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.clone(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

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
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = 10_000_000;
    let withdrawal_limit = 10_000;
    let withdrawal_daily_limit = 1_000;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload: Vec<u8> = vec![];
    let attached_amount = 0;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_sol_request_ix(
            funder.pubkey(),
            author.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            mint_address,
            round_number,
            recipient,
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.mint, mint_address);
    assert_eq!(withdrawal_data.event.data.recipient, recipient);
    assert_eq!(withdrawal_data.event.data.amount, amount);

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );
    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, None)
    );

    // Check Proposal Account to unpack
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );
}

#[tokio::test]
async fn test_withdraw_sol_request_with_fake_payload() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Round Account
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.clone(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

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
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = 10_000_000;
    let withdrawal_limit = 10_000;
    let withdrawal_daily_limit = 1_000;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload = bincode::serialize(&vec![solana_program::system_instruction::allocate(
        &Pubkey::new_unique(),
        0,
    )])
    .unwrap();

    let attached_amount = 5;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_sol_request_ix(
            funder.pubkey(),
            author.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            mint_address,
            round_number,
            recipient,
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.mint, mint_address);
    assert_eq!(withdrawal_data.event.data.recipient, recipient);
    assert_eq!(withdrawal_data.event.data.amount, amount);

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let (_, proxy_nonce) = Pubkey::find_program_address(
        &[br"proxy", &mint_address.to_bytes(), &recipient.to_bytes()],
        &token_proxy::id(),
    );

    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, Some(proxy_nonce))
    );

    // Check Proposal Account to unpack
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );

    // Check Proxy Account
    let proxy_address = get_proxy_address(&mint_address, &recipient);

    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    assert_eq!(proxy_info.data.len(), spl_token::state::Account::LEN);
}

#[tokio::test]
async fn test_vote_for_withdrawal_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Relay Accounts
    let relays = vec![
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
    ];

    for relay in &relays {
        program_test.add_account(
            relay.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: solana_program::system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    // Add Relay Round Account
    let round_number = 7;
    let round_ttl = 1209600;

    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let mint = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: author.pubkey(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: (relays.len() * 2 / 3 + 1) as u32,
        signers: relays.iter().map(|_| Vote::None).collect(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + RELAY_REPARATION * relays.len() as u64,
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    // Vote for withdrawal request
    for relay in &relays {
        let mut transaction = Transaction::new_with_payer(
            &[vote_for_withdrawal_request_ix(
                relay.pubkey(),
                withdrawal_address,
                round_number,
                Vote::Confirm,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &relay], recent_blockhash);

        let _ = banks_client.process_transaction(transaction).await;
    }

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal unpack");

    let sig_count = withdrawal_data
        .signers
        .iter()
        .filter(|vote| **vote == Vote::Confirm)
        .count();

    assert_eq!(sig_count, relays.len());
}

#[tokio::test]
async fn test_create_token_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Recipient Account
    let recipient = Keypair::new();
    program_test.add_account(
        recipient.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let mint = get_mint_address(&token);

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient.pubkey(), &mint);

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient.pubkey(),
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenEverEventWithLen::new(
        token,
        name,
        symbol,
        decimals,
        amount,
        recipient.pubkey(),
        payload,
    );
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let signers = vec![Vote::Confirm; 3];

    let withdrawal_account_data = WithdrawalMultiTokenEver {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: signers.len() as u32,
        signers: signers.clone(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenEver::LEN];
    WithdrawalMultiTokenEver::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenEver::LEN)
                + Rent::default().minimum_balance(TokenSettings::LEN)
                + Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[create_ever_token_ix(
            funder.pubkey(),
            withdrawal_address,
            recipient.pubkey(),
            token,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Token Settings Account
    let token_settings_address = get_token_settings_ever_address(&token);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("deposit token unpack");

    assert_eq!(token_settings_data.is_initialized, true);
    assert_eq!(token_settings_data.deposit_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_limit, u64::MAX);
    assert_eq!(token_settings_data.emergency, false);

    assert_eq!(
        token_settings_data.kind,
        TokenKind::Ever {
            mint,
            token,
            decimals,
        }
    );

    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());
    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", token_hash.as_ref()], &token_proxy::id());

    assert_eq!(
        token_settings_data.account_kind,
        AccountKind::TokenSettings(token_settings_nonce, mint_nonce)
    );

    let fee_info = &token_settings_data.fee_info;

    let fee = 1.max(
        (amount as u64)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount as u64 - fee;

    assert_eq!(token_settings_data.withdrawal_daily_amount, transfer_amount);

    // Check Mint Supply
    let mint_info = banks_client
        .get_account(mint)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, transfer_amount);

    // Check Recipient Account
    let recipient_token_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data = spl_token::state::Account::unpack(recipient_token_info.data())
        .expect("recipient token unpack");
    assert_eq!(recipient_token_data.amount, transfer_amount);

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );
}

#[tokio::test]
async fn test_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        amount: 100,
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

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let fee_info = token_settings_account_data.fee_info.clone();

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let signers = vec![Vote::Confirm; 3];

    let withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: signers.len() as u32,
        signers: signers.clone(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + Rent::default().minimum_balance(TokenSettings::LEN)
                + Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_sol_ix(
            withdrawal_address,
            token_wallet,
            mint_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Vault Balance
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");

    let fee = 1.max(
        (amount as u64)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount as u64 - fee;

    assert_eq!(vault_data.amount, 100 - transfer_amount);

    // Check Recipient Balance
    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("recipient token unpack");
    assert_eq!(recipient_data.amount, transfer_amount);

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );
}

#[tokio::test]
async fn test_change_guardian() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_guardian = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[change_guardian_ix(owner.pubkey(), new_guardian)],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.guardian, new_guardian);
}

#[tokio::test]
async fn test_change_manager() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_manager = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[change_manager_ix(owner.pubkey(), new_manager)],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.manager, new_manager);
}

#[tokio::test]
async fn test_change_withdrawal_manager() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_withdrawal_manager = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[change_withdrawal_manager_ix(
            owner.pubkey(),
            new_withdrawal_manager,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.withdrawal_manager, new_withdrawal_manager);
}

#[tokio::test]
async fn test_change_deposit_limit() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        manager: manager.pubkey(),
        guardian,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_deposit_limit = 1_000_000;

    let mut transaction = Transaction::new_with_payer(
        &[change_deposit_limit_ix(
            manager.pubkey(),
            token_settings_address,
            new_deposit_limit,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.deposit_limit, new_deposit_limit);
}

#[tokio::test]
async fn test_change_withdrawal_limits() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let manager = Pubkey::new_unique();
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: false,
        manager,
        guardian,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_withdrawal_limit = 10000000;
    let new_withdrawal_daily_limit = 12345;

    let mut transaction = Transaction::new_with_payer(
        &[change_withdrawal_limits_by_owner_ix(
            owner.pubkey(),
            token_settings_address,
            Some(new_withdrawal_limit),
            Some(new_withdrawal_daily_limit),
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.withdrawal_limit, new_withdrawal_limit);
    assert_eq!(
        token_settings_data.withdrawal_daily_limit,
        new_withdrawal_daily_limit
    );
}

#[tokio::test]
async fn test_enable_emergency() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Keypair::new();
    let manager = Pubkey::new_unique();

    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian: guardian.pubkey(),
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[enable_emergency_ix(guardian.pubkey())],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &guardian], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.emergency, true);
}

#[tokio::test]
async fn test_enable_emergency_by_owner() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[enable_emergency_by_owner_ix(owner.pubkey())],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.emergency, true);
}

#[tokio::test]
async fn test_disable_emergency() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: true,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[disable_emergency_ix(owner.pubkey())],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.emergency, false);
}

#[tokio::test]
async fn test_enable_token_emergency() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Keypair::new();
    let manager = Pubkey::new_unique();

    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian: guardian.pubkey(),
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], &token_proxy::id());

    let token_settings_address = get_token_settings_sol_address(&mint);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, 0),
        kind: TokenKind::Solana { mint, vault },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[enable_emergency_token_ix(
            guardian.pubkey(),
            token_settings_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &guardian], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.emergency, true);
}

#[tokio::test]
async fn test_disable_token_emergency() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let (programdata_address, programdata_nonce) =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id());

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(owner.pubkey()),
    };

    let programdata_data_serialized =
        bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap();

    program_test.add_account(
        programdata_address,
        Account {
            lamports: Rent::default().minimum_balance(programdata_data_serialized.len()),
            data: programdata_data_serialized,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, programdata_nonce),
        emergency: true,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let mint = Pubkey::new_unique();
    let vault = Pubkey::new_unique();
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], &token_proxy::id());

    let token_settings_address = get_token_settings_sol_address(&mint);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, programdata_nonce),
        kind: TokenKind::Solana { mint, vault },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: true,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[disable_emergency_token_ix(
            owner.pubkey(),
            token_settings_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.emergency, false);
}

#[tokio::test]
async fn test_approve_withdrawal_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let withdrawal_manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager: withdrawal_manager.pubkey(),
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenEverEventWithLen::new(
        token, name, symbol, decimals, amount, recipient, payload,
    );
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let signers = vec![Vote::Confirm; 3];

    let mut withdrawal_account_data = WithdrawalMultiTokenEver {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: signers.len() as u32,
        signers: signers.clone(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };
    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForApprove;

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenEver::LEN];
    WithdrawalMultiTokenEver::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenEver::LEN)
                + Rent::default().minimum_balance(TokenSettings::LEN)
                + Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[approve_withdrawal_ever_ix(
            withdrawal_manager.pubkey(),
            withdrawal_address,
            token_wallet,
            mint_address,
            &token,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &withdrawal_manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );

    let mint_info = banks_client
        .get_account(mint_address)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, amount as u64);

    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount as u64);
}

#[tokio::test]
async fn test_approve_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let withdrawal_manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager: withdrawal_manager.pubkey(),
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        amount: 100,
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

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let signers = vec![Vote::Confirm; 3];

    let mut withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: signers.len() as u32,
        signers: signers.clone(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };
    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForApprove;

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + Rent::default().minimum_balance(TokenSettings::LEN)
                + Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[approve_withdrawal_sol_ix(
            withdrawal_manager.pubkey(),
            withdrawal_address,
            token_wallet,
            mint_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &withdrawal_manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("settings unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");
    assert_eq!(vault_data.amount, 100 - amount as u64);

    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount as u64);
}

#[tokio::test]
async fn test_update_fee() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        withdrawal_manager,
        manager: manager.pubkey(),
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let mint_address = Pubkey::new_unique();

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let multiplier = 1;
    let divisor = 100;

    let mut transaction = Transaction::new_with_payer(
        &[update_fee_ix(
            manager.pubkey(),
            token_settings_address,
            multiplier,
            divisor,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.fee_info.multiplier, multiplier);
    assert_eq!(token_settings_data.fee_info.divisor, divisor);
}

#[tokio::test]
async fn test_withdrawal_ever_fee() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        withdrawal_manager,
        manager: manager.pubkey(),
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", &token_hash.as_ref()], &token_proxy::id());

    let mint_address = get_mint_address(&token);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;
    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());

    let token_settings_address = get_token_settings_ever_address(&token);

    let fee_supply = 1_000_000;

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
        kind: TokenKind::Ever {
            mint: mint_address,
            token,
            decimals,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: FeeInfo {
            multiplier: 5,
            divisor: 10_000,
            supply: fee_supply,
        },
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_ever_fee_ix(
            manager.pubkey(),
            mint_address,
            token_wallet,
            &token,
            fee_supply,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let mint_info = banks_client
        .get_account(mint_address)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, fee_supply);

    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, fee_supply);

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.fee_info.supply, 0);
}

#[tokio::test]
async fn test_withdrawal_sol_fee() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Keypair::new();

    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        withdrawal_manager,
        manager: manager.pubkey(),
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    let fee_supply = 100;

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        amount: fee_supply,
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

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: FeeInfo {
            multiplier: 1,
            divisor: 1,
            supply: fee_supply,
        },
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_sol_fee_ix(
            manager.pubkey(),
            token_wallet,
            mint_address,
            fee_supply,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &manager], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Vault Balance
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");

    assert_eq!(vault_data.amount, 0);

    // Check Recipient Balance
    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("recipient token unpack");

    assert_eq!(recipient_data.amount, fee_supply);

    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("token settings unpack");

    assert_eq!(token_settings_data.fee_info.supply, 0);
}

#[tokio::test]
async fn test_change_bounty_for_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    let author = Keypair::new();

    // Add Withdrawal Account
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let round_number = 1;
    let mint = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let mut withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: author.pubkey(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: 1,
        signers: vec![Vote::Confirm],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };
    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + RELAY_REPARATION,
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let bounty = 5;
    let mut transaction = Transaction::new_with_payer(
        &[change_bounty_for_withdrawal_sol_ix(
            &author.pubkey(),
            &withdrawal_address,
            bounty,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal unpack");
    assert_eq!(withdrawal_data.meta.data.bounty, bounty);
}

#[tokio::test]
async fn test_cancel_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Pubkey::new_unique();
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        manager,
        guardian,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

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
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let round_number = 1;
    let recipient = Pubkey::new_unique();
    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let mut withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: author.pubkey(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: 1,
        signers: vec![Vote::Confirm],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };
    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + RELAY_REPARATION,
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let mut transaction = Transaction::new_with_payer(
        &[cancel_withdrawal_sol_ix(
            funder.pubkey(),
            author.pubkey(),
            withdrawal_address,
            mint_address,
            deposit_seed,
            recipient,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Cancelled
    );

    let new_deposit_address = get_deposit_address(deposit_seed);
    let new_deposit_info = banks_client
        .get_account(new_deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenSol::unpack(new_deposit_info.data()).expect("deposit unpack");
    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed);
}

#[tokio::test]
async fn test_fill_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let manager = Pubkey::new_unique();
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        manager,
        guardian,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Token Account
    let author_token_address =
        spl_associated_token_account::get_associated_token_address(&author.pubkey(), &mint_address);

    let author_token_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: author.pubkey(),
        amount: 100,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut author_token_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(author_token_account_data, &mut author_token_packed).unwrap();
    program_test.add_account(
        author_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: author_token_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_token_address = spl_associated_token_account::get_associated_token_address(
        &recipient_address,
        &mint_address,
    );

    let recipient_token_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_token_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_token_account_data, &mut recipient_token_packed)
        .unwrap();
    program_test.add_account(
        recipient_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_token_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let round_number = 1;
    let amount = 32;
    let bounty = 2;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient_address,
        amount,
        payload.clone(),
    );

    let event =
        WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient_address, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let mut withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author: author.pubkey(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: 1,
        signers: vec![Vote::Confirm],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };
    withdrawal_account_data.meta.data.bounty = bounty;
    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + RELAY_REPARATION,
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let ever_recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let mut transaction = Transaction::new_with_payer(
        &[fill_withdrawal_sol_ix(
            funder.pubkey(),
            author.pubkey(),
            recipient_address,
            mint_address,
            withdrawal_address,
            deposit_seed,
            ever_recipient,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );

    let author_token_info = banks_client
        .get_account(author_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let author_token_data =
        spl_token::state::Account::unpack(author_token_info.data()).expect("sender unpack");
    assert_eq!(author_token_data.amount, 100 - amount as u64 + bounty);

    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("recipient unpack");
    assert_eq!(recipient_token_data.amount, amount as u64 - bounty);

    let deposit_address = get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositMultiTokenSol::unpack(deposit_info.data()).expect("deposit unpack");

    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed);
}

#[tokio::test]
async fn test_withdraw_sol_with_payload() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Accounts
    let relays = vec![
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
    ];

    for relay in &relays {
        program_test.add_account(
            relay.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: solana_program::system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    // Add Relay Round Account
    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        state: AccountState::Initialized,
        amount: 100,
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
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = 10_000_000;
    let withdrawal_limit = 10_000;
    let withdrawal_daily_limit = 1_000;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let fee_info = token_settings_account_data.fee_info.clone();

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Recipient Token Account
    let recipient = Keypair::new();
    let recipient_token_address = spl_associated_token_account::get_associated_token_address(
        &recipient.pubkey(),
        &mint_address,
    );

    let recipient_token_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient.pubkey(),
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_token_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_token_account_data, &mut recipient_token_packed)
        .unwrap();
    program_test.add_account(
        recipient_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_token_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    // Create withdrawal request
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let amount = 32;

    let (proxy_address, proxy_nonce) = Pubkey::find_program_address(
        &[
            br"proxy",
            &mint_address.to_bytes(),
            &recipient.pubkey().to_bytes(),
        ],
        &token_proxy::id(),
    );

    let payload = bincode::serialize(&vec![spl_token::instruction::transfer(
        &spl_token::id(),
        &proxy_address,
        &recipient_token_address,
        &proxy_address,
        &[&proxy_address],
        16,
    )
    .unwrap()])
    .unwrap();

    let attached_amount = 0;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_sol_request_ix(
            funder.pubkey(),
            author.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            mint_address,
            round_number,
            recipient.pubkey(),
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient.pubkey(),
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.mint, mint_address);
    assert_eq!(withdrawal_data.event.data.recipient, recipient.pubkey());
    assert_eq!(withdrawal_data.event.data.amount, amount);

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, Some(proxy_nonce))
    );

    // Check Proposal Account to unpack
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );

    // Vote for withdrawal request
    for relay in &relays {
        let mut transaction = Transaction::new_with_payer(
            &[vote_for_withdrawal_request_ix(
                relay.pubkey(),
                withdrawal_address,
                round_number,
                Vote::Confirm,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &relay], recent_blockhash);

        let _ = banks_client.process_transaction(transaction).await;
    }

    // Execute withdrawal
    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_sol_with_payload_ix(
            withdrawal_address,
            recipient.pubkey(),
            mint_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Vault Balance
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");

    let fee = 1.max(
        (amount as u64)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount as u64 - fee;

    assert_eq!(vault_data.amount, 100 - transfer_amount);

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, transfer_amount);

    // Withdrawal token from Proxy Account
    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_proxy_ix(
            recipient.pubkey(),
            recipient_token_address,
            mint_address,
            15,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &recipient], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, 16);

    // Check Recipient Balance
    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("proxy unpack");
    assert_eq!(recipient_token_data.amount, 15);

    // Execute payload
    let data = TokenProxyInstruction::ExecutePayloadSol
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(withdrawal_address, false),
            AccountMeta::new(proxy_address, false),
            AccountMeta::new(recipient_token_address, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    let mut transaction = Transaction::new_with_payer(&[ix], Some(&funder.pubkey()));
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, 0);

    // Check Proxy Balance
    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("proxy unpack");
    assert_eq!(recipient_token_data.amount, transfer_amount);

    // Check status
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );
}

#[tokio::test]
async fn test_withdraw_ever_request_with_payload() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Round Loader Settings Account
    let round_number = 12;

    let rl_settings_address = get_associated_settings_address(&round_loader::id());

    let (_, rl_settings_nonce) = Pubkey::find_program_address(&[br"settings"], &round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(rl_settings_nonce, 0),
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 0,
    };

    let mut rl_settings_packed = vec![0; round_loader::Settings::LEN];
    round_loader::Settings::pack(rl_settings_account_data, &mut rl_settings_packed).unwrap();
    program_test.add_account(
        rl_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::Settings::LEN),
            data: rl_settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Relay Accounts
    let relays = vec![
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
    ];

    for relay in &relays {
        program_test.add_account(
            relay.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: solana_program::system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    // Add Relay Round Account
    let relay_round_address =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let (_, relay_round_nonce) = Pubkey::find_program_address(
        &[br"relay_round", &round_number.to_le_bytes()],
        &round_loader::id(),
    );

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound(relay_round_nonce),
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
        round_number,
        round_end,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(round_loader::RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Author Account
    let author = Keypair::new();
    program_test.add_account(
        author.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let mint = get_mint_address(&token);

    let recipient = Keypair::new();
    let recipient_token_address =
        spl_associated_token_account::get_associated_token_address(&recipient.pubkey(), &mint);

    let amount = 32;

    let (proxy_address, proxy_nonce) = Pubkey::find_program_address(
        &[br"proxy", &mint.to_bytes(), &recipient.pubkey().to_bytes()],
        &token_proxy::id(),
    );

    let payload = bincode::serialize(&vec![spl_token::instruction::transfer(
        &spl_token::id(),
        &proxy_address,
        &recipient_token_address,
        &proxy_address,
        &[&proxy_address],
        16,
    )
    .unwrap()])
    .unwrap();

    let attached_amount = 0;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_ever_request_ix(
            funder.pubkey(),
            author.pubkey(),
            round_number,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            token,
            name.clone(),
            symbol.clone(),
            decimals,
            recipient.pubkey(),
            amount,
            payload.clone(),
            attached_amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_address = get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient.pubkey(),
        amount,
        payload,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.author, author.pubkey());
    assert_eq!(withdrawal_data.round_number, round_number);

    assert_eq!(
        withdrawal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.token, token);
    assert_eq!(withdrawal_data.event.data.name, name);
    assert_eq!(withdrawal_data.event.data.symbol, symbol);
    assert_eq!(withdrawal_data.event.data.decimals, decimals);
    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.event.data.recipient, recipient.pubkey());

    assert_ne!(withdrawal_data.meta.data.epoch, 0);
    assert_eq!(withdrawal_data.meta.data.bounty, 0);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    let event_data = hash(&withdrawal_data.event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    assert_eq!(
        withdrawal_data.account_kind,
        AccountKind::Proposal(withdrawal_nonce, Some(proxy_nonce))
    );

    // Check Proposal Account
    let proposal_data =
        Proposal::unpack_from_slice(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        proposal_data.event,
        withdrawal_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        proposal_data.meta,
        withdrawal_data.meta.data.try_to_vec().unwrap()
    );

    // Check Proxy Account
    let proxy_address = get_proxy_address(&mint, &recipient.pubkey());

    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    assert_eq!(proxy_info.data.len(), spl_token::state::Account::LEN);

    // Vote for withdrawal request
    for relay in &relays {
        let mut transaction = Transaction::new_with_payer(
            &[vote_for_withdrawal_request_ix(
                relay.pubkey(),
                withdrawal_address,
                round_number,
                Vote::Confirm,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &relay], recent_blockhash);

        let _ = banks_client.process_transaction(transaction).await;
    }

    // Execute withdrawal
    let mut transaction = Transaction::new_with_payer(
        &[create_ever_token_with_payload_ix(
            funder.pubkey(),
            withdrawal_address,
            recipient.pubkey(),
            token,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Token Settings Account
    let token_settings_address = get_token_settings_ever_address(&token);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("deposit token unpack");

    assert_eq!(token_settings_data.is_initialized, true);
    assert_eq!(token_settings_data.deposit_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_limit, u64::MAX);
    assert_eq!(token_settings_data.emergency, false);

    assert_eq!(
        token_settings_data.kind,
        TokenKind::Ever {
            mint,
            token,
            decimals,
        }
    );

    let token_hash = hash(&token.try_to_vec().unwrap());

    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], &token_proxy::id());
    let (_, mint_nonce) =
        Pubkey::find_program_address(&[br"mint", token_hash.as_ref()], &token_proxy::id());

    assert_eq!(
        token_settings_data.account_kind,
        AccountKind::TokenSettings(token_settings_nonce, mint_nonce)
    );

    let fee_info = &token_settings_data.fee_info;

    let fee = 1.max(
        (amount as u64)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount as u64 - fee;

    assert_eq!(token_settings_data.withdrawal_daily_amount, transfer_amount);

    // Check Mint supply
    let mint_info = banks_client
        .get_account(mint)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, transfer_amount);

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, transfer_amount);

    // Withdrawal token from Proxy Account
    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_proxy_ix(
            recipient.pubkey(),
            recipient_token_address,
            mint,
            15,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &recipient], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, 16);

    // Check Recipient Balance
    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("proxy unpack");
    assert_eq!(recipient_token_data.amount, 15);

    // Execute payload
    let data = TokenProxyInstruction::ExecutePayloadEver
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(withdrawal_address, false),
            AccountMeta::new(proxy_address, false),
            AccountMeta::new(recipient_token_address, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    let mut transaction = Transaction::new_with_payer(&[ix], Some(&funder.pubkey()));
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Proxy Balance
    let proxy_info = banks_client
        .get_account(proxy_address)
        .await
        .expect("get_account")
        .expect("account");

    let proxy_data = spl_token::state::Account::unpack(proxy_info.data()).expect("proxy unpack");
    assert_eq!(proxy_data.amount, 0);

    // Check Recipient Balance
    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("proxy unpack");
    assert_eq!(recipient_token_data.amount, transfer_amount);

    // Check status
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );
}

#[tokio::test]
async fn close_withdrawal() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let decimals = spl_token::native_mint::DECIMALS;

    let mint_address = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint_address.to_bytes()], &token_proxy::id());

    let vault_address = get_vault_address(&mint_address);

    let vault_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: vault_address,
        amount: 100,
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

    // Add Recipient Token Account
    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_address);

    let token_wallet_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut token_wallet_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(token_wallet_account_data, &mut token_wallet_packed).unwrap();
    program_test.add_account(
        token_wallet,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: token_wallet_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Token Settings Account
    let symbol = "USDT".to_string();
    let name = "USDT Solana Octusbridge".to_string();
    let deposit_limit = u64::MAX;
    let withdrawal_limit = u64::MAX;
    let withdrawal_daily_limit = u64::MAX;

    let (_, token_settings_nonce) = Pubkey::find_program_address(
        &[br"settings", &mint_address.to_bytes()],
        &token_proxy::id(),
    );

    let token_settings_address = get_token_settings_sol_address(&mint_address);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
        kind: TokenKind::Solana {
            mint: mint_address,
            vault: vault_address,
        },
        name,
        symbol,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
        fee_info: Default::default(),
    };

    let fee_info = token_settings_account_data.fee_info.clone();

    let mut token_settings_packed = vec![0; TokenSettings::LEN];
    TokenSettings::pack(token_settings_account_data, &mut token_settings_packed).unwrap();
    program_test.add_account(
        token_settings_address,
        Account {
            lamports: Rent::default().minimum_balance(TokenSettings::LEN),
            data: token_settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let amount = 32;

    let payload: Vec<u8> = vec![];

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
        payload.clone(),
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient, payload);
    let event_data = hash(&event.data.try_to_vec().expect("pack")).to_bytes();

    let (_, withdrawal_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data,
        ],
        &token_proxy::id(),
    );

    let signers = vec![Vote::Confirm; 3];

    // Add Author Account
    let author = Pubkey::new_unique();
    program_test.add_account(
        author,
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let withdrawal_account_data = WithdrawalMultiTokenSol {
        is_initialized: true,
        account_kind: AccountKind::Proposal(withdrawal_nonce, None),
        author,
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::default(),
        required_votes: signers.len() as u32,
        signers: signers.clone(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalMultiTokenSol::LEN];
    WithdrawalMultiTokenSol::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalMultiTokenSol::LEN)
                + Rent::default().minimum_balance(TokenSettings::LEN)
                + Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_sol_ix(
            withdrawal_address,
            token_wallet,
            mint_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Vault Balance
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");

    let fee = 1.max(
        (amount as u64)
            .checked_div(fee_info.divisor)
            .unwrap()
            .checked_mul(fee_info.multiplier)
            .unwrap(),
    );

    let transfer_amount = amount as u64 - fee;

    assert_eq!(vault_data.amount, 100 - transfer_amount);

    // Check Recipient Balance
    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("recipient token unpack");
    assert_eq!(recipient_data.amount, transfer_amount);

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Processed
    );

    // Close Withdrawal
    let mut transaction = Transaction::new_with_payer(
        &[close_withdrawal_ix(withdrawal_address, author)],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account");

    assert_eq!(withdrawal_info, None);
}

#[tokio::test]
async fn test_close_deposit() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce, 0),
        emergency: false,
        guardian,
        manager,
        withdrawal_manager,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Mint Account
    let mint = Pubkey::new_unique();

    let decimals = spl_token::native_mint::DECIMALS;

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add MultiVault Account
    let (_, multivault_nonce) = Pubkey::find_program_address(&[br"multivault"], &token_proxy::id());

    let multivault_address = get_multivault_address();

    let multivault_account_data = MultiVault {
        is_initialized: true,
        account_kind: AccountKind::MultiVault(multivault_nonce),
    };

    let mut multivault_packed = vec![0; MultiVault::LEN];
    MultiVault::pack(multivault_account_data, &mut multivault_packed).unwrap();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: Rent::default().minimum_balance(MultiVault::LEN),
            data: multivault_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Account
    let sender = Keypair::new();

    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint);

    let sender_account_data = spl_token::state::Account {
        mint,
        owner: sender.pubkey(),
        amount: 100,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut sender_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(sender_account_data, &mut sender_packed).unwrap();
    program_test.add_account(
        sender_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: sender_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4().as_u128();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let value = 1000;
    let payload: Vec<u8> = vec![];
    let expected_evers = UInt256::default();
    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_sol_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            mint,
            deposit_seed,
            name.clone(),
            symbol.clone(),
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
    let vault_address = get_vault_address(&mint);

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("vault unpack");
    assert_eq!(vault_data.amount, amount);

    // Check Sender Valance
    let sender_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 100 - amount);

    // Check Token Settings Account
    let token_settings_address = get_token_settings_sol_address(&mint);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings_data =
        TokenSettings::unpack(token_settings_info.data()).expect("deposit token unpack");

    assert_eq!(token_settings_data.is_initialized, true);
    assert_eq!(token_settings_data.withdrawal_epoch, 0);
    assert_eq!(token_settings_data.deposit_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_limit, u64::MAX);
    assert_eq!(token_settings_data.withdrawal_daily_amount, 0);
    assert_eq!(token_settings_data.emergency, false);

    assert_eq!(
        token_settings_data.kind,
        TokenKind::Solana {
            mint,
            vault: vault_address
        }
    );

    let (_, token_settings_nonce) =
        Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], &token_proxy::id());
    let (_, vault_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint.to_bytes()], &token_proxy::id());

    assert_eq!(
        token_settings_data.account_kind,
        AccountKind::TokenSettings(token_settings_nonce, vault_nonce)
    );

    // Check Deposit Account
    let deposit_address = get_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenSol::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[br"deposit", &deposit_seed.to_le_bytes()],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.base_token, mint);
    assert_eq!(deposit_data.event.data.name, name);
    assert_eq!(deposit_data.event.data.symbol, symbol);
    assert_eq!(deposit_data.event.data.decimals, decimals);
    assert_eq!(deposit_data.event.data.value, value);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

    let fee_info = &token_settings_data.fee_info;
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
        Deposit::unpack_from_slice(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(
        raw_deposit_data.event,
        deposit_data.event.data.try_to_vec().unwrap()
    );
    assert_eq!(
        raw_deposit_data.meta,
        deposit_data.meta.data.try_to_vec().unwrap()
    );

    // Close Deposit
    let mut transaction = Transaction::new_with_payer(
        &[close_deposit_ix(sender.pubkey(), deposit_address)],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Deposit Account
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account");

    assert_eq!(deposit_info, None);
}
