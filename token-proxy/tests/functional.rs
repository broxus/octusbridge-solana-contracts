#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::{EverAddress, Vote, RELAY_REPARATION};

use bridge_utils::helper::get_associated_relay_round_address;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::hash::hash;
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

    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        AccountKind::Settings(settings_nonce)
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
        account_kind: AccountKind::Settings(settings_nonce),
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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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
    let sol_amount = 1000;
    let payload: Vec<u8> = vec![];

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_ever_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            &token,
            deposit_seed,
            recipient,
            amount,
            sol_amount,
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
    let deposit_address = get_deposit_address(deposit_seed, &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenEver::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &token_settings_address.to_bytes(),
        ],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.token, token);
    assert_eq!(deposit_data.event.data.amount, amount as u128);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);
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
        account_kind: AccountKind::Settings(settings_nonce),
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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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
    let sol_amount = 1000;
    let payload: Vec<u8> = vec![];

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_ever_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            &token,
            deposit_seed,
            recipient,
            amount,
            sol_amount,
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
    let deposit_address = get_deposit_address(deposit_seed, &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenEver::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &token_settings_address.to_bytes(),
        ],
        &token_proxy::id(),
    );
    assert_eq!(
        deposit_data.account_kind,
        AccountKind::Deposit(deposit_nonce)
    );

    assert_eq!(deposit_data.event.data.token, token);
    assert_eq!(
        deposit_data.event.data.amount,
        (amount * 1_000_000_000) as u128
    );
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

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
        account_kind: AccountKind::Settings(settings_nonce),
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
    let sol_amount = 1000;
    let payload: Vec<u8> = vec![];
    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_sol_ix(
            funder.pubkey(),
            sender.pubkey(),
            sender_associated_token_address,
            mint,
            deposit_seed,
            recipient,
            amount,
            name.clone(),
            symbol.clone(),
            sol_amount,
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
    let deposit_address = get_deposit_address(deposit_seed, &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenSol::unpack(deposit_info.data()).expect("deposit token unpack");

    assert_eq!(deposit_data.is_initialized, true);

    let (_, deposit_nonce) = Pubkey::find_program_address(
        &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &token_settings_address.to_bytes(),
        ],
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
    assert_eq!(deposit_data.event.data.amount, amount as u128);
    assert_eq!(deposit_data.event.data.sol_amount, sol_amount);
    assert_eq!(deposit_data.event.data.recipient, recipient);
    assert_eq!(deposit_data.event.data.payload, payload);

    assert_eq!(deposit_data.meta.data.seed, deposit_seed);

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
        account_kind: AccountKind::Settings(rl_settings_nonce),
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

    let relay_round_address = get_associated_relay_round_address(&round_loader::id(), round_number);

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
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.is_executed, false);
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
        AccountKind::Proposal(withdrawal_nonce)
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
        account_kind: AccountKind::Settings(rl_settings_nonce),
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

    let relay_round_address = get_associated_relay_round_address(&round_loader::id(), round_number);

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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.is_executed, false);
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
        AccountKind::Proposal(withdrawal_nonce)
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

    let relay_round_address = get_associated_relay_round_address(&round_loader::id(), round_number);

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

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint,
        recipient,
        amount,
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint, amount, recipient);
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
        account_kind: AccountKind::Proposal(withdrawal_nonce),
        is_executed: false,
        author: author.pubkey(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, 0),
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
        .count() as u32;

    assert_eq!(sig_count, withdrawal_data.required_votes);
}

#[tokio::test]
async fn test_withdrawal_ever() {
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
        account_kind: AccountKind::Settings(settings_nonce),
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

    // Add Recipient Token Account
    let token = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let mint = get_mint_address(&token);

    let recipient = Pubkey::new_unique();

    let token_wallet =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint);

    let token_wallet_account_data = spl_token::state::Account {
        mint,
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

    // Add Withdrawal Account
    let round_number = 7;

    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let name = "USDC ETHEREUM OCTUSBRIDGE".to_string();
    let symbol = "USDC".to_string();
    let decimals = spl_token::native_mint::DECIMALS;

    let amount = 32;

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
    );

    let event =
        WithdrawalMultiTokenEverEventWithLen::new(token, name, symbol, decimals, amount, recipient);
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
        account_kind: AccountKind::Proposal(withdrawal_nonce),
        is_executed: false,
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, 0),
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
        &[withdrawal_ever_ix(
            funder.pubkey(),
            withdrawal_address,
            token_wallet,
            token,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Mint Supply
    let mint_info = banks_client
        .get_account(mint)
        .await
        .expect("get_account")
        .expect("account");

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(mint_data.supply, amount as u64);

    // Check Recipient Balance
    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("recipient token unpack");
    assert_eq!(recipient_data.amount, amount as u64);

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");
    assert_eq!(withdrawal_data.is_executed, true);

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
        account_kind: AccountKind::Settings(settings_nonce),
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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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

    let withdrawal_address = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient,
        amount,
    );

    let event = WithdrawalMultiTokenSolEventWithLen::new(mint_address, amount, recipient);
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
        account_kind: AccountKind::Proposal(withdrawal_nonce),
        is_executed: false,
        author: Pubkey::new_unique(),
        round_number,
        event,
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, 0),
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
    assert_eq!(vault_data.amount, 100 - amount as u64);

    // Check Recipient Balance
    let recipient_info = banks_client
        .get_account(token_wallet)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("recipient token unpack");
    assert_eq!(recipient_data.amount, amount as u64);

    // Check Withdrawal Account
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");
    assert_eq!(withdrawal_data.is_executed, true);

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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        account_kind: AccountKind::Settings(settings_nonce),
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
        &[change_guardian_ix(&owner.pubkey(), new_guardian)],
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        account_kind: AccountKind::Settings(settings_nonce),
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
        &[change_manager_ix(&owner.pubkey(), new_manager)],
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        account_kind: AccountKind::Settings(settings_nonce),
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
            &owner.pubkey(),
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
    let owner = Keypair::new();

    // Add Program Data Account
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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
            &owner.pubkey(),
            &token_settings_address,
            new_deposit_limit,
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        emergency: false,
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
        &[change_withdrawal_limits_ix(
            &owner.pubkey(),
            &token_settings_address,
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
        account_kind: AccountKind::Settings(settings_nonce),
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
        &[enable_emergency_ix(&guardian.pubkey())],
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        account_kind: AccountKind::Settings(settings_nonce),
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
        &[enable_emergency_by_owner_ix(&owner.pubkey())],
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
        account_kind: AccountKind::Settings(settings_nonce),
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
        &[disable_emergency_ix(&owner.pubkey())],
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

/*#[tokio::test]
async fn test_enable_token_emergency() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let guardian = Keypair::new();
    let withdrawal_manager = Pubkey::new_unique();

    // Add Settings Account
    let guardian = Pubkey::new_unique();
    let manager = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();
    let (_, settings_nonce) = Pubkey::find_program_address(&[br"settings"], &token_proxy::id());

    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings(settings_nonce),
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

    // Add Token Settings Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;

    let mint_address = get_mint_address(&name);

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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
        &[enable_token_emergency_ix(&guardian.pubkey(), &name)],
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
        TokenSettings::unpack(token_settings_info.data()).expect("token_settings unpack");

    assert_eq!(token_settings_data.emergency, true);
}

#[tokio::test]
async fn test_enable_token_emergency_by_owner() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    // Add Program Data Account
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
    let withdrawal_manager = Pubkey::new_unique();

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;

    let mint_address = get_mint_address(&name);

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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
        &[enable_token_emergency_by_owner_ix(&owner.pubkey(), &name)],
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
        TokenSettings::unpack(token_settings_info.data()).expect("token_settings unpack");

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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
    let withdrawal_manager = Pubkey::new_unique();

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;

    let mint_address = get_mint_address(&name);

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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
        &[disable_token_emergency_ix(&owner.pubkey(), &name)],
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
        TokenSettings::unpack(token_settings_info.data()).expect("token_settings unpack");

    assert_eq!(token_settings_data.emergency, false);
}*/

/*
#[tokio::test]
async fn test_approve_withdrawal_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let withdrawal_manager = Keypair::new();

    let mint_address = get_mint_address(&name);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals: solana_decimals,
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

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
        guardian: Pubkey::new_unique(),
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(
            &recipient_address,
            &mint_address,
        );

    let recipient_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let round_number = 5;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
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
            &withdrawal_manager.pubkey(),
            &recipient_address,
            &withdrawal_address,
            &name,
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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal unpack");
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
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount as u64);
}

#[tokio::test]
async fn test_approve_withdrawal_ever_by_owner() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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

    // Add Mint Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        decimals: solana_decimals,
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

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(
            &recipient_address,
            &mint_address,
        );

    let recipient_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let round_number = 5;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[approve_withdrawal_ever_by_owner_ix(
            &owner.pubkey(),
            &recipient_address,
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal unpack");
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
        .get_account(recipient_associated_token_address)
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
    let withdrawal_manager = Keypair::new();
    let mint = Pubkey::new_unique();

    let name = "USDT".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let vault_address = get_vault_address(&name);
    let guardian = Pubkey::new_unique();

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
        guardian,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Mint Account
    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint),
        decimals: solana_decimals,
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

    // Add Vault Account
    let vault_address = get_vault_address(&name);

    let vault_account_data = spl_token::state::Account {
        mint,
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
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&recipient_address, &mint);

    let recipient_account_data = spl_token::state::Account {
        mint,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let round_number = 5;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
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
            &withdrawal_manager.pubkey(),
            &recipient_address,
            &mint,
            &withdrawal_address,
            &name,
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

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("settings unpack");
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
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount as u64);
}

#[tokio::test]
async fn test_approve_withdrawal_sol_2() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let withdrawal_manager = Keypair::new();
    let mint = Pubkey::new_unique();

    let name = "USDT".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let vault_address = get_vault_address(&name);

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
        guardian,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Mint Account
    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint),
        decimals: solana_decimals,
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

    // Add Vault Account
    let vault_address = get_vault_address(&name);

    let vault_account_data = spl_token::state::Account {
        mint,
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
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&recipient_address, &mint);

    let recipient_account_data = spl_token::state::Account {
        mint,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let round_number = 5;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 1000;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
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
            &withdrawal_manager.pubkey(),
            &recipient_address,
            &mint,
            &withdrawal_address,
            &name,
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

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("settings unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Pending
    );
}

#[tokio::test]
async fn test_approve_withdrawal_sol_by_owner() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let owner = Keypair::new();

    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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

    let mint = Pubkey::new_unique();

    let name = "USDT".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let vault_address = get_vault_address(&name);

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Mint Account
    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint),
        decimals: solana_decimals,
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

    // Add Vault Account
    let vault_address = get_vault_address(&name);

    let vault_account_data = spl_token::state::Account {
        mint,
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
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(&recipient_address, &mint);

    let recipient_account_data = spl_token::state::Account {
        mint,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let round_number = 5;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[approve_withdrawal_sol_by_owner_ix(
            &owner.pubkey(),
            &recipient_address,
            &mint,
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &owner], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("settings unpack");
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
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount as u64);
}

#[tokio::test]
async fn test_cancel_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint.pubkey()),
        decimals: solana_decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint.pubkey(),
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Vault Account
    let vault_address = get_vault_address(&name);

    let vault_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
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

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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
    let round_number = 7;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let recipient_address = Pubkey::new_unique();
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author: author.pubkey(),
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4();
    let recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let mut transaction = Transaction::new_with_payer(
        &[cancel_withdrawal_sol_ix(
            &funder.pubkey(),
            &author.pubkey(),
            &withdrawal_address,
            deposit_seed,
            &name,
            Some(recipient_address),
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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal unpack");

    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Cancelled
    );

    let new_deposit_address = get_deposit_address(deposit_seed.as_u128(), &token_settings_address);
    let new_deposit_info = banks_client
        .get_account(new_deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(new_deposit_info.data()).expect("deposit unpack");
    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.event.data.recipient_address, recipient_address);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_fill_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();
    let decimals = 9;

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint.pubkey()),
        decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint.pubkey(),
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
    let author_token_address = spl_associated_token_account::get_associated_token_address(
        &author.pubkey(),
        &mint.pubkey(),
    );

    let author_token_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
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
        &mint.pubkey(),
    );

    let recipient_token_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
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

    let name = "USTD".to_string();
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let solana_decimals = 9;
    let ever_decimals = 9;

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: Pubkey::new_unique(),
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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
    let withdrawal_author = Pubkey::new_unique();
    let round_number = 9;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;
    let bounty = 1;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author: withdrawal_author,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, bounty, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
            data: withdrawal_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4();
    let ever_recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let mut transaction = Transaction::new_with_payer(
        &[fill_withdrawal_sol_ix(
            &funder.pubkey(),
            &author.pubkey(),
            &recipient_address,
            &mint.pubkey(),
            &withdrawal_address,
            &name,
            deposit_seed,
            ever_recipient_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

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

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(deposit_info.data()).expect("deposit unpack");

    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_change_bounty_for_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    let name = "USDT".to_string();
    let token_settings_address = get_token_settings_address(&name);

    // Add Withdrawal Account
    let author = Keypair::new();
    let round_number = 9;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let recipient_address = Pubkey::new_unique();

    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &token_settings_address,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: true,
        author: author.pubkey(),
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, 0, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: token_settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN),
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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal unpack");
    assert_eq!(withdrawal_data.meta.data.bounty, bounty);
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
    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

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
    let withdrawal_manager = Pubkey::new_unique();

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let new_guardian = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[change_guardian_ix(&owner.pubkey(), new_guardian)],
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
async fn test_deposit_multi_token_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals: solana_decimals,
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

    // Add Sender Account
    let sender = Keypair::new();
    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 100000000,
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

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Multi Vault Account
    let multivault_address = get_multivault_address();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4();
    let recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let token_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let sol_amount = 100;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint_address);

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_ever_ix(
            &funder.pubkey(),
            &sender.pubkey(),
            &author_token_pubkey,
            &name,
            deposit_seed,
            recipient_address,
            token_address,
            amount,
            sol_amount,
            vec![],
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenEver::unpack(deposit_info.data()).expect("deposit token unpack");
    assert_eq!(deposit_data.event.data.amount as u64, amount);
    assert_eq!(deposit_data.event.data.sol_amount as u64, sol_amount);
    assert_eq!(deposit_data.event.data.recipient_address, recipient_address);
    assert_eq!(deposit_data.event.data.token_address, token_address);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_deposit_multi_token_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "OCTUSBRIDGE ETHEREUM USDT".to_string();
    let symbol = "USDT".to_string();
    let solana_decimals = 9;
    let guardian = Pubkey::new_unique();
    let withdrawal_manager = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint_address),
        supply: 100,
        decimals: solana_decimals,
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

    // Add Sender Account
    let sender = Keypair::new();
    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 100000000,
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

    // Add Settings Account
    let settings_address = get_settings_address();

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
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

    // Add Multi Vault Account
    let multivault_address = get_multivault_address();
    program_test.add_account(
        multivault_address,
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let deposit_seed = uuid::Uuid::new_v4();
    let recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;
    let sol_amount = 100;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&sender.pubkey(), &mint_address);

    let mut transaction = Transaction::new_with_payer(
        &[deposit_multi_token_sol_ix(
            &funder.pubkey(),
            &sender.pubkey(),
            &author_token_pubkey,
            deposit_seed,
            recipient_address,
            amount,
            name.clone(),
            symbol.clone(),
            sol_amount,
            vec![],
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);
    let token_settings_info = banks_client
        .get_account(token_settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let token_settings =
        TokenSettings::unpack(token_settings_info.data()).expect("deposit token unpack");
    assert_eq!(token_settings.is_initialized, true);
    assert_eq!(token_settings.name, name);
    assert_eq!(token_settings.solana_decimals, solana_decimals);
    assert_eq!(token_settings.ever_decimals, solana_decimals);

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &token_settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data =
        DepositMultiTokenSol::unpack(deposit_info.data()).expect("deposit token unpack");
    assert_eq!(deposit_data.event.data.amount as u64, amount);
    assert_eq!(deposit_data.event.data.symbol, symbol);
    assert_eq!(deposit_data.event.data.sol_amount as u64, sol_amount);
    assert_eq!(deposit_data.event.data.recipient_address, recipient_address);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_withdrawal_multi_token_ever_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let name = "WEVER".to_string();
    let symbol = "OCTUSBRIDGE WRAPPED EVER".to_string();
    let ever_decimals = 9;

    let mint_address = get_mint_address(&name);

    // Add Round Loader Settings Account
    let round_number = 12;
    let rl_settings_address =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
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
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
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

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(
            &recipient_address,
            &mint_address,
        );

    let recipient_account_data = spl_token::state::Account {
        mint: mint_address,
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
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
            lamports: 100000000,
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
    let token_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_ever_request_ix(
            &funder.pubkey(),
            &author.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            token_address,
            name.clone(),
            symbol.clone(),
            ever_decimals,
            round_number,
            recipient_address,
            amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_pubkey = get_token_settings_address(&name);

    let withdrawal_address = get_multivault_withdrawal_ever_address(
        &token_settings_pubkey,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token_address,
        name.clone(),
        symbol.clone(),
        ever_decimals,
        recipient_address,
        amount,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenEver::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);

    assert_eq!(withdrawal_data.pda.settings, token_settings_pubkey);
    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    assert_eq!(withdrawal_data.signers.len(), relays.len());
    for (i, _) in relays.iter().enumerate() {
        assert_eq!(withdrawal_data.signers[i], Vote::None);
    }
}

#[tokio::test]
async fn test_withdrawal_multi_token_sol_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let solana_decimals = 9;
    let ever_decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;

    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint.pubkey()),
        decimals: solana_decimals,
        ..Default::default()
    };

    let mut mint_packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut mint_packed).unwrap();
    program_test.add_account(
        mint.pubkey(),
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: mint_packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Add Token Settings Account
    let token_settings_address = get_token_settings_address(&name);

    let token_settings_account_data = TokenSettings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: Pubkey::new_unique(),
        },
        withdrawal_daily_amount: 0,
        withdrawal_epoch: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        solana_decimals,
        ever_decimals,
        emergency: false,
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

    // Add Round Loader Settings Account
    let round_number = 12;
    let rl_settings_address =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let round_ttl = 1209600;
    let rl_settings_account_data = round_loader::Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
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
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let round_end = round_ttl + chrono::Utc::now().timestamp() as u32;

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
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

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        spl_associated_token_account::get_associated_token_address(
            &recipient_address,
            &mint.pubkey(),
        );

    let recipient_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
        owner: recipient_address,
        state: AccountState::Initialized,
        ..Default::default()
    };

    let mut recipient_packed = vec![0; spl_token::state::Account::LEN];
    spl_token::state::Account::pack(recipient_account_data, &mut recipient_packed).unwrap();
    program_test.add_account(
        recipient_associated_token_address,
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Account::LEN),
            data: recipient_packed,
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
            lamports: 100000000,
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
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_multi_token_sol_request_ix(
            &funder.pubkey(),
            &author.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            mint.pubkey(),
            name.clone(),
            round_number,
            recipient_address,
            amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let token_settings_pubkey = get_token_settings_address(&name);

    let withdrawal_address = get_multivault_withdrawal_sol_address(
        &token_settings_pubkey,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint.pubkey(),
        recipient_address,
        amount,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalMultiTokenSol::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);

    assert_eq!(withdrawal_data.pda.settings, token_settings_pubkey);
    assert_eq!(withdrawal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(
        withdrawal_data.pda.event_transaction_lt,
        event_transaction_lt
    );
    assert_eq!(withdrawal_data.pda.event_configuration, event_configuration);

    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    assert_eq!(withdrawal_data.signers.len(), relays.len());
    for (i, _) in relays.iter().enumerate() {
        assert_eq!(withdrawal_data.signers[i], Vote::None);
    }
}*/
