#![cfg(feature = "test-bpf")]

use bridge_utils::state::{AccountKind, PDA};
use bridge_utils::types::{EverAddress, Vote, RELAY_REPARATION};

use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::rent::Rent;
use solana_program::{bpf_loader_upgradeable, program_option, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::AccountState;

use token_proxy::*;

#[tokio::test]
async fn test_init_mint() {
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

    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();
    let mut transaction = Transaction::new_with_payer(
        &[initialize_mint_ix(
            &funder.pubkey(),
            &initializer.pubkey(),
            name.clone(),
            decimals,
            deposit_limit,
            withdrawal_limit,
            withdrawal_daily_limit,
            admin,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &initializer], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let mint_address = get_mint_address(&name);
    let mint_info = banks_client
        .get_account(mint_address)
        .await
        .expect("get_account")
        .expect("account");

    assert_eq!(mint_info.owner, spl_token::id());

    let mint_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");

    assert_eq!(mint_data.is_initialized, true);
    assert_eq!(mint_data.decimals, decimals);
    assert_eq!(mint_data.supply, 0);
    assert_eq!(
        mint_data.mint_authority,
        program_option::COption::Some(mint_address)
    );

    let settings_address = get_settings_address(&name);
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.emergency, false);
    assert_eq!(settings_data.name, name);
    assert_eq!(settings_data.kind, TokenKind::Ever { mint: mint_address });
    assert_eq!(settings_data.deposit_limit, deposit_limit);
    assert_eq!(settings_data.withdrawal_limit, withdrawal_limit);
    assert_eq!(settings_data.admin, admin);
}

#[tokio::test]
async fn test_init_vault() {
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

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();
    let mint_account_data = spl_token::state::Mint {
        is_initialized: true,
        mint_authority: program_option::COption::Some(mint.pubkey()),
        decimals,
        ..Default::default()
    };

    let mut packed = vec![0; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint_account_data, &mut packed).unwrap();
    program_test.add_account(
        mint.pubkey(),
        Account {
            lamports: Rent::default().minimum_balance(spl_token::state::Mint::LEN),
            data: packed,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 1,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[initialize_vault_ix(
            &funder.pubkey(),
            &initializer.pubkey(),
            &mint.pubkey(),
            name.clone(),
            deposit_limit,
            withdrawal_limit,
            withdrawal_daily_limit,
            admin,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &initializer], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let vault_address = get_vault_address(&name);
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    assert_eq!(vault_info.owner, spl_token::id());

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");

    assert_eq!(vault_data.mint, mint.pubkey());
    assert_eq!(vault_data.owner, vault_address);
    assert_eq!(vault_data.state, AccountState::Initialized);
    assert_eq!(vault_data.amount, 0);

    let settings_address = get_settings_address(&name);
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.name, name);
    assert_eq!(settings_data.emergency, false);
    assert_eq!(
        settings_data.kind,
        TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        }
    );
    assert_eq!(settings_data.deposit_limit, deposit_limit);
    assert_eq!(settings_data.withdrawal_limit, withdrawal_limit);
    assert_eq!(settings_data.admin, admin);
}

#[tokio::test]
async fn test_deposit_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
        name: name.clone(),
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    let deposit_seed = uuid::Uuid::new_v4();
    let recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[deposit_ever_ix(
            &funder.pubkey(),
            &sender.pubkey(),
            &name,
            deposit_seed,
            recipient_address,
            amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

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
    assert_eq!(mint_data.supply, 100 - amount);

    let sender_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_data = spl_token::state::Account::unpack(sender_info.data()).expect("token unpack");
    assert_eq!(sender_data.amount, 100 - amount);

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(deposit_info.data()).expect("deposit token unpack");
    assert_eq!(
        deposit_data.event.data.sender_address,
        sender.pubkey().to_bytes().to_vec()
    );
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.event.data.recipient_address, recipient_address);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_deposit_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

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

    // Add Vault Account
    let vault_address = get_vault_address(&name);

    let vault_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
        owner: vault_address,
        amount: 0,
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
        spl_associated_token_account::get_associated_token_address(
            &sender.pubkey(),
            &mint.pubkey(),
        );

    let sender_account_data = spl_token::state::Account {
        mint: mint.pubkey(),
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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        emergency: false,
        name: name.clone(),
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    let deposit_seed = uuid::Uuid::new_v4();
    let recipient_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[deposit_sol_ix(
            &funder.pubkey(),
            &sender.pubkey(),
            &mint.pubkey(),
            &name,
            deposit_seed,
            recipient_address,
            amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let vault_address = get_vault_address(&name);
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");
    assert_eq!(vault_data.amount, amount);

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &settings_address);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(deposit_info.data()).expect("deposit token unpack");
    assert_eq!(
        deposit_data.event.data.sender_address,
        sender.pubkey().to_bytes().to_vec()
    );
    assert_eq!(deposit_data.event.data.amount, amount);
    assert_eq!(deposit_data.event.data.recipient_address, recipient_address);
    assert_eq!(deposit_data.meta.data.seed, deposit_seed.as_u128());
}

#[tokio::test]
async fn test_withdrawal_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let name = "WEVER".to_string();
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    // Add Relay Round Account
    let round_number = 12;
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let relay_round_address =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let relay_round_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
        round_end: 1946154867,
        relays: relays.clone(),
        round_number,
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
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_request_ix(
            &funder.pubkey(),
            &author.pubkey(),
            &settings_address,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            round_number,
            sender_address,
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

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data =
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal token unpack");

    assert_eq!(withdrawal_data.is_initialized, true);

    assert_eq!(withdrawal_data.pda.settings, settings_address);
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
async fn test_vote_for_withdrawal_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let name = "WEVER".to_string();
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    // Add Relays Accounts
    let relays = vec![Keypair::new(), Keypair::new(), Keypair::new()];
    let relay_init_lamports = 100000;
    for relay in &relays {
        program_test.add_account(
            relay.pubkey(),
            Account {
                lamports: relay_init_lamports,
                data: vec![],
                owner: solana_program::system_program::id(),
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    let round_number = 12;
    let relay_round_address =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let relay_round_account_data = round_loader::RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
        round_end: 1946154867,
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
        round_number,
    };

    let mut relay_round_packed = vec![0; round_loader::RelayRound::LEN];
    round_loader::RelayRound::pack(relay_round_account_data, &mut relay_round_packed).unwrap();
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

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let recipient_address = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
        required_votes: relays.len() as u32,
        signers: relays.iter().map(|_| Vote::None).collect(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
        },
    };

    let mut withdrawal_packed = vec![0; WithdrawalToken::LEN];
    WithdrawalToken::pack(withdrawal_account_data, &mut withdrawal_packed).unwrap();
    program_test.add_account(
        withdrawal_address,
        Account {
            lamports: Rent::default().minimum_balance(WithdrawalToken::LEN) + RELAY_REPARATION * 3,
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
                &relay.pubkey(),
                &withdrawal_address,
                round_number,
                Vote::Confirm,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &relay], recent_blockhash);

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");
    }

    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("mint unpack");
    assert_eq!(withdrawal_data.signers.len(), relays.len());
    for (i, _) in relays.iter().enumerate() {
        assert_eq!(withdrawal_data.signers[i], Vote::Confirm);
    }

    assert_eq!(
        withdrawal_info.lamports,
        Rent::default().minimum_balance(WithdrawalToken::LEN)
    );

    for relay in &relays {
        let relay_info = banks_client
            .get_account(relay.pubkey())
            .await
            .expect("get_account")
            .expect("account");

        assert_eq!(relay_info.lamports, relay_init_lamports + RELAY_REPARATION);
    }
}

#[tokio::test]
async fn test_withdrawal_ever() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

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

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
        &[withdrawal_ever_ix(
            &recipient_address,
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

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
    assert_eq!(mint_data.supply, amount);

    let recipient_info = banks_client
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount);
}

#[tokio::test]
async fn test_withdrawal_ever_2() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = get_mint_address(&name);

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

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 1001;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
        &[withdrawal_ever_ix(
            &recipient_address,
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal token unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::WaitingForApprove
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

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
        &[withdrawal_sol_ix(
            &recipient_address,
            &mint.pubkey(),
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");
    assert_eq!(vault_data.amount, 100 - amount);

    let recipient_info = banks_client
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount);
}

#[tokio::test]
async fn test_withdrawal_sol_2() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 101;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
        &[withdrawal_sol_ix(
            &recipient_address,
            &mint.pubkey(),
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

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
        WithdrawalToken::unpack(withdrawal_info.data()).expect("withdrawal token unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::Pending
    );
}

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
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Keypair::new();

    let mint_address = get_mint_address(&name);

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

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin: admin.pubkey(),
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
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
            &admin.pubkey(),
            &recipient_address,
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &admin], recent_blockhash);

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
    assert_eq!(mint_data.supply, amount);

    let recipient_info = banks_client
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount);
}

#[tokio::test]
async fn test_approve_withdrawal_sol() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let admin = Keypair::new();
    let mint = Pubkey::new_unique();

    let name = "USDT".to_string();
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let vault_address = get_vault_address(&name);

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin: admin.pubkey(),
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

    // Add Withdrawal Account
    let author = Pubkey::new_unique();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let recipient_address = Pubkey::new_unique();
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::WaitingForApprove, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
            &admin.pubkey(),
            &withdrawal_address,
            &name,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &admin], recent_blockhash);

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
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let recipient_address = Pubkey::new_unique();
    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author: author.pubkey(),
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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

    let mut transaction = Transaction::new_with_payer(
        &[cancel_withdrawal_sol_ix(
            &funder.pubkey(),
            &author.pubkey(),
            &withdrawal_address,
            deposit_seed,
            &name,
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

    let new_deposit_address = get_deposit_address(deposit_seed.as_u128(), &settings_address);
    let new_deposit_info = banks_client
        .get_account(new_deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(new_deposit_info.data()).expect("deposit unpack");
    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
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
    let admin = Pubkey::new_unique();

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: Pubkey::new_unique(),
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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

    // Add Withdrawal Account
    let withdrawal_author = Pubkey::new_unique();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;
    let bounty = 1;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author: withdrawal_author,
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, bounty),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
    assert_eq!(author_token_data.amount, 100 - amount + bounty);

    let recipient_token_info = banks_client
        .get_account(recipient_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_token_data =
        spl_token::state::Account::unpack(recipient_token_info.data()).expect("recipient unpack");
    assert_eq!(recipient_token_data.amount, amount - bounty);

    let deposit_address = get_deposit_address(deposit_seed.as_u128(), &settings_address);
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
async fn test_transfer_from_vault() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let mint = Keypair::new();

    let name = "USDT".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Keypair::new();

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
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin: admin.pubkey(),
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

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let amount = 10;

    let mut transaction = Transaction::new_with_payer(
        &[transfer_from_vault_ix(
            &admin.pubkey(),
            &mint.pubkey(),
            &recipient_address,
            name,
            amount,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &admin], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");
    assert_eq!(vault_data.amount, 100 - amount);

    let recipient_info = banks_client
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_data =
        spl_token::state::Account::unpack(recipient_info.data()).expect("token unpack");
    assert_eq!(recipient_data.amount, amount);
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
    let settings_address = get_settings_address(&name);

    // Add Withdrawal Account
    let author = Keypair::new();
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let recipient_address = Pubkey::new_unique();

    let amount = 10;

    let withdrawal_address = get_withdrawal_address(
        &settings_address,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        author: author.pubkey(),
        account_kind: AccountKind::Proposal,
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
        meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::Pending, 0),
        required_votes: 0,
        signers: vec![],
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings: settings_address,
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
async fn test_change_settings() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Mint Account
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Keypair::new();

    let mint_address = get_mint_address(&name);

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

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin: admin.pubkey(),
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

    let new_emergency = true;
    let new_deposit_limit = 10000000;
    let new_withdrawal_limit = 10000;
    let new_withdrawal_daily_limit = 1000;

    let mut transaction = Transaction::new_with_payer(
        &[change_settings_ix(
            &admin.pubkey(),
            name,
            new_emergency,
            new_deposit_limit,
            new_withdrawal_limit,
            new_withdrawal_daily_limit,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &admin], recent_blockhash);

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

    assert_eq!(settings_data.emergency, new_emergency);
    assert_eq!(settings_data.deposit_limit, new_deposit_limit);
    assert_eq!(settings_data.withdrawal_limit, new_withdrawal_limit);
    assert_eq!(
        settings_data.withdrawal_daily_limit,
        new_withdrawal_daily_limit
    );
}

#[tokio::test]
async fn test_change_admin() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let authority = Keypair::new();

    let programdata_address =
        Pubkey::find_program_address(&[token_proxy::id().as_ref()], &bpf_loader_upgradeable::id())
            .0;

    let programdata_data = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(authority.pubkey()),
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
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Keypair::new();

    let mint_address = get_mint_address(&name);

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

    // Add Settings Account
    let settings_address = get_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        name: name.clone(),
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin: admin.pubkey(),
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

    let new_admin = Pubkey::new_unique();

    let mut transaction = Transaction::new_with_payer(
        &[change_admin_ix(&authority.pubkey(), &name, new_admin)],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &authority], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_address = get_settings_address(&name);
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.admin, new_admin);
}
