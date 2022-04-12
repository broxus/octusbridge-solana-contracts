#![cfg(feature = "test-bpf")]

use bridge_utils::EverAddress;
use rand::Rng;
use round_loader::RelayRound;

use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::rent::Rent;
use solana_program::{bpf_loader_upgradeable, program_option, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::AccountState;

use token_proxy::{
    get_associated_deposit_address, get_associated_withdrawal_address, DepositToken, Processor,
    Settings, TokenKind, WithdrawalToken, WithdrawalTokenEventWithLen, WithdrawalTokenMetaWithLen,
    WithdrawalTokenStatus,
};

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
        &[token_proxy::initialize_mint(
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

    let mint_address = token_proxy::get_associated_mint_address(&name);
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

    let settings_address = token_proxy::get_associated_settings_address(&name);
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.emergency, false);
    assert_eq!(settings_data.kind, TokenKind::Ever { mint: mint_address });
    assert_eq!(settings_data.decimals, decimals);
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
        &[token_proxy::initialize_vault(
            &funder.pubkey(),
            &initializer.pubkey(),
            &mint.pubkey(),
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

    let vault_address = token_proxy::get_associated_vault_address(&name);
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

    let settings_address = token_proxy::get_associated_settings_address(&name);
    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.emergency, false);
    assert_eq!(
        settings_data.kind,
        TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        }
    );
    assert_eq!(settings_data.decimals, decimals);
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

    let mint_address = token_proxy::get_associated_mint_address(&name);

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
            lamports: 0,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        get_associated_token_address(&sender.pubkey(), &mint_address);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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

    let deposit_seed = rand::thread_rng().gen::<u64>();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[token_proxy::deposit_ever(
            &funder.pubkey(),
            &sender.pubkey(),
            name,
            recipient,
            amount,
            deposit_seed,
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
    let vault_address = token_proxy::get_associated_vault_address(&name);

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
            lamports: 0,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        get_associated_token_address(&sender.pubkey(), &mint.pubkey());

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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

    let deposit_seed = rand::thread_rng().gen::<u64>();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[token_proxy::deposit_sol(
            &funder.pubkey(),
            &mint.pubkey(),
            &sender.pubkey(),
            name.clone(),
            recipient,
            amount,
            deposit_seed,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let vault_address = token_proxy::get_associated_vault_address(&name);
    let vault_info = banks_client
        .get_account(vault_address)
        .await
        .expect("get_account")
        .expect("account");

    let vault_data = spl_token::state::Account::unpack(vault_info.data()).expect("mint unpack");
    assert_eq!(vault_data.amount, amount);
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
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = token_proxy::get_associated_mint_address(&name);

    // Add Settings Account
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
    let settings_address = round_loader::get_associated_relay_round_address(round_number);

    let relay_round_account_data = RelayRound {
        is_initialized: true,
        round_ttl: 1946154867,
        relays: relays.clone(),
        round_number,
    };

    let mut relay_round_packed = vec![0; RelayRound::LEN];
    RelayRound::pack(relay_round_account_data, &mut relay_round_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_packed,
            owner: token_proxy::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        get_associated_token_address(&recipient_address, &mint_address);

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

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let author = Keypair::new();
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 32;

    let mut transaction = Transaction::new_with_payer(
        &[token_proxy::withdrawal_request(
            &funder.pubkey(),
            &author.pubkey(),
            name,
            round_number,
            &event_configuration,
            event_transaction_lt,
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

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("mint unpack");

    assert_eq!(withdrawal_data.is_initialized, true);
    assert_eq!(withdrawal_data.event.data.amount, amount);
    assert_eq!(withdrawal_data.meta.data.status, WithdrawalTokenStatus::New);

    assert_eq!(withdrawal_data.kind, TokenKind::Ever { mint: mint_address });

    assert_eq!(withdrawal_data.signers.len(), relays.len());
    for (i, _) in relays.iter().enumerate() {
        assert_eq!(withdrawal_data.signers[i], token_proxy::Vote::None);
    }
}

#[tokio::test]
async fn test_confirm_withdrawal_request() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = token_proxy::get_associated_mint_address(&name);

    // Add Settings Account
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
    let relay = Keypair::new();

    let round_number = 12;
    let settings_address = round_loader::get_associated_relay_round_address(round_number);

    let relay_round_account_data = RelayRound {
        is_initialized: true,
        round_ttl: 1946154867,
        relays: vec![relay.pubkey()],
        round_number,
    };

    let mut relay_round_packed = vec![0; RelayRound::LEN];
    RelayRound::pack(relay_round_account_data, &mut relay_round_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Withdrawal Round Account
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let recipient_address = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Ever { mint: mint_address },
        round_number,
        event: WithdrawalTokenEventWithLen::new(decimals, recipient_address, sender_address, 10),
        meta: WithdrawalTokenMetaWithLen::new(Pubkey::new_unique(), WithdrawalTokenStatus::New, 0),
        required_votes: 1,
        signers: vec![token_proxy::Vote::None],
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
        &[token_proxy::vote_for_withdrawal_request(
            &relay.pubkey(),
            round_number,
            &event_configuration,
            event_transaction_lt,
            token_proxy::Vote::Confirm,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &relay], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("mint unpack");
    assert_eq!(withdrawal_data.signers[0], token_proxy::Vote::Confirm);
}

#[tokio::test]
async fn test_update_withdrawal_status() {
    let mut program_test = ProgramTest::new(
        "token_proxy",
        token_proxy::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let name = "WEVER".to_string();
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let admin = Pubkey::new_unique();

    let mint_address = token_proxy::get_associated_mint_address(&name);

    // Add Settings Account
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let recipient_address = Pubkey::new_unique();
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Ever { mint: mint_address },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(decimals, recipient_address, sender_address, 10),
        meta: WithdrawalTokenMetaWithLen::new(Pubkey::new_unique(), WithdrawalTokenStatus::New, 0),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::update_withdrawal_status(
            name,
            &event_configuration,
            event_transaction_lt,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);
    let withdrawal_info = banks_client
        .get_account(withdrawal_address)
        .await
        .expect("get_account")
        .expect("account");

    let withdrawal_data = WithdrawalToken::unpack(withdrawal_info.data()).expect("mint unpack");
    assert_eq!(
        withdrawal_data.meta.data.status,
        WithdrawalTokenStatus::WaitingForRelease
    );
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

    let mint_address = token_proxy::get_associated_mint_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        get_associated_token_address(&recipient_address, &mint_address);

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
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Ever { mint: mint_address },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            recipient_address,
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::WaitingForRelease,
            0,
        ),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::withdrawal_ever(
            &recipient_address,
            name,
            &event_configuration,
            event_transaction_lt,
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

    let withdrawal_data = spl_token::state::Mint::unpack(mint_info.data()).expect("mint unpack");
    assert_eq!(withdrawal_data.supply, amount);

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
    let vault_address = token_proxy::get_associated_vault_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        get_associated_token_address(&recipient_address, &mint.pubkey());

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
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            recipient_address,
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::WaitingForRelease,
            0,
        ),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::withdrawal_sol(
            &mint.pubkey(),
            &recipient_address,
            name,
            &event_configuration,
            event_transaction_lt,
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

    let mint_address = token_proxy::get_associated_mint_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        get_associated_token_address(&recipient_address, &mint_address);

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

    // Add Withdrawal Round Account
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Ever { mint: mint_address },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            recipient_address,
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::WaitingForApprove,
            0,
        ),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::approve_withdrawal_ever(
            &admin.pubkey(),
            &recipient_address,
            name,
            &event_configuration,
            event_transaction_lt,
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
    let decimals = 9;
    let deposit_limit = 10000000;
    let withdrawal_limit = 10000;
    let withdrawal_daily_limit = 1000;
    let vault_address = token_proxy::get_associated_vault_address(&name);

    // Add Settings Account
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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

    // Add Withdrawal Round Account
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint,
            vault: vault_address,
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            Pubkey::new_unique(),
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::WaitingForApprove,
            0,
        ),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::approve_withdrawal_sol(
            &admin.pubkey(),
            name,
            &event_configuration,
            event_transaction_lt,
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
    let vault_address = token_proxy::get_associated_vault_address(&name);

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

    // Add Withdrawal Round Account
    let author = Keypair::new();
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            Pubkey::new_unique(),
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(author.pubkey(), WithdrawalTokenStatus::Pending, 0),
        required_votes: 0,
        signers: vec![],
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

    let deposit_seed = rand::thread_rng().gen::<u64>();

    let mut transaction = Transaction::new_with_payer(
        &[token_proxy::cancel_withdrawal_sol(
            &funder.pubkey(),
            &author.pubkey(),
            &event_configuration,
            event_transaction_lt,
            deposit_seed,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let withdrawal_address =
        get_associated_withdrawal_address(&event_configuration, event_transaction_lt);
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

    let new_deposit_address = get_associated_deposit_address(deposit_seed);
    let new_deposit_info = banks_client
        .get_account(new_deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(new_deposit_info.data()).expect("deposit unpack");
    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
}

#[tokio::test]
async fn test_force_withdrawal_sol() {
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
    let vault_address = token_proxy::get_associated_vault_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        get_associated_token_address(&recipient_address, &mint.pubkey());

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

    // Add Withdrawal Round Account
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            recipient_address,
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::Pending,
            0,
        ),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::force_withdrawal_sol(
            &mint.pubkey(),
            &recipient_address,
            name,
            &event_configuration,
            event_transaction_lt,
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

    // Add Sender Account
    let sender = Keypair::new();
    program_test.add_account(
        sender.pubkey(),
        Account {
            lamports: 0,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Add Sender Token Account
    let sender_associated_token_address =
        get_associated_token_address(&sender.pubkey(), &mint.pubkey());

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

    // Add Recipient Token Account
    let recipient_address = Pubkey::new_unique();
    let recipient_associated_token_address =
        get_associated_token_address(&recipient_address, &mint.pubkey());

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
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());
    let amount = 10;
    let bounty = 1;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: Pubkey::new_unique(),
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            recipient_address,
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(
            Pubkey::new_unique(),
            WithdrawalTokenStatus::Pending,
            bounty,
        ),
        required_votes: 0,
        signers: vec![],
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

    let deposit_seed = rand::thread_rng().gen::<u64>();
    let recipient = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let mut transaction = Transaction::new_with_payer(
        &[token_proxy::fill_withdrawal_sol(
            &funder.pubkey(),
            &sender.pubkey(),
            &mint.pubkey(),
            &recipient_address,
            &event_configuration,
            event_transaction_lt,
            deposit_seed,
            recipient,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &sender], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let sender_associated_token_info = banks_client
        .get_account(sender_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let sender_associated_token_data =
        spl_token::state::Account::unpack(sender_associated_token_info.data())
            .expect("sender unpack");
    assert_eq!(sender_associated_token_data.amount, 100 - amount + bounty);

    let recipient_associated_token_info = banks_client
        .get_account(recipient_associated_token_address)
        .await
        .expect("get_account")
        .expect("account");

    let recipient_associated_token_data =
        spl_token::state::Account::unpack(recipient_associated_token_info.data())
            .expect("recipient unpack");
    assert_eq!(recipient_associated_token_data.amount, amount - bounty);

    let deposit_address = get_associated_deposit_address(deposit_seed);
    let deposit_info = banks_client
        .get_account(deposit_address)
        .await
        .expect("get_account")
        .expect("account");

    let deposit_data = DepositToken::unpack(deposit_info.data()).expect("deposit unpack");

    assert_eq!(deposit_data.is_initialized, true);
    assert_eq!(deposit_data.event.data.amount, amount);
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
    let vault_address = token_proxy::get_associated_vault_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Solana {
            mint: mint.pubkey(),
            vault: vault_address,
        },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        get_associated_token_address(&recipient_address, &mint.pubkey());

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
        &[token_proxy::transfer_from_vault(
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

    // Add Withdrawal Account
    let author = Keypair::new();
    let event_configuration = ton_types::UInt256::new();
    let event_transaction_lt = 123;
    let sender_address = EverAddress::with_standart(0, Pubkey::new_unique().to_bytes());

    let decimals = 9;
    let amount = 10;

    let withdrawal_address =
        token_proxy::get_associated_withdrawal_address(&event_configuration, event_transaction_lt);

    let withdrawal_account_data = WithdrawalToken {
        is_initialized: true,
        kind: TokenKind::Solana {
            mint: Pubkey::new_unique(),
            vault: Pubkey::new_unique(),
        },
        round_number: 5,
        event: WithdrawalTokenEventWithLen::new(
            decimals,
            Pubkey::new_unique(),
            sender_address,
            amount,
        ),
        meta: WithdrawalTokenMetaWithLen::new(author.pubkey(), WithdrawalTokenStatus::Pending, 0),
        required_votes: 0,
        signers: vec![],
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
        &[token_proxy::change_bounty_for_withdrawal_sol(
            &author.pubkey(),
            &event_configuration,
            event_transaction_lt,
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

    let mint_address = token_proxy::get_associated_mint_address(&name);

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
    let settings_address = token_proxy::get_associated_settings_address(&name);

    let settings_account_data = Settings {
        is_initialized: true,
        emergency: false,
        kind: TokenKind::Ever { mint: mint_address },
        withdrawal_daily_amount: 0,
        withdrawal_ttl: 0,
        decimals,
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
        &[token_proxy::change_settings(
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
