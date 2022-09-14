#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::types::{Vote, RELAY_REPARATION};
use std::str::FromStr;

use bridge_utils::state::{AccountKind, PDA};
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::rent::Rent;
use solana_program::{bpf_loader_upgradeable, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use round_loader::*;

#[tokio::test]
async fn test_init_relay_loader() {
    let mut program_test = ProgramTest::new(
        "round_loader",
        round_loader::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let initializer = Keypair::new();
    program_test.add_account(
        initializer.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let programdata_address = Pubkey::find_program_address(
        &[round_loader::id().as_ref()],
        &bpf_loader_upgradeable::id(),
    )
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
            data: bincode::serialize::<UpgradeableLoaderState>(&programdata_data).unwrap(),
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let creator = Keypair::new();

    let round_submitter = creator.pubkey();
    let genesis_round_number = 0;
    let min_required_votes = 1;

    let round_number = 1;
    let round_ttl = 1209600;
    let round_end = chrono::Utc::now().timestamp();
    let relays = vec![Pubkey::from_str("2Xzby8BnopnMbCS12YgASrxJoemVFJFgSbSB8pbU1am3").unwrap()];

    let mut transaction = Transaction::new_with_payer(
        &[
            initialize_ix(
                &funder.pubkey(),
                &initializer.pubkey(),
                genesis_round_number,
                round_submitter,
                min_required_votes,
                round_ttl,
            ),
            create_relay_round_ix(
                &funder.pubkey(),
                &creator.pubkey(),
                round_number,
                round_end as u32,
                relays.clone(),
            ),
        ],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &creator, &initializer], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_address = get_settings_address();

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.current_round_number, round_number);
    assert_eq!(settings_data.round_submitter, round_submitter);
    assert_eq!(settings_data.min_required_votes, min_required_votes);
    assert_eq!(settings_data.round_ttl, round_ttl);

    let relay_round_address = get_relay_round_address(round_number);

    let relay_round_info = banks_client
        .get_account(relay_round_address)
        .await
        .expect("get_account")
        .expect("account");

    let relay_round_data = RelayRound::unpack(relay_round_info.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_number, round_number);
    assert_eq!(relay_round_data.round_end, round_end as u32 + round_ttl);
    assert_eq!(relay_round_data.relays, relays);

    let new_current_round_number = 3;
    let new_round_submitter = Pubkey::new_unique();
    let new_min_required_votes = 12;

    let mut transaction = Transaction::new_with_payer(
        &[update_settings_ix(
            &initializer.pubkey(),
            Some(new_current_round_number),
            Some(new_round_submitter),
            Some(new_min_required_votes),
            None,
        )],
        Some(&initializer.pubkey()),
    );
    transaction.sign(&[&initializer], recent_blockhash);

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

    assert_eq!(settings_data.current_round_number, new_current_round_number);
    assert_eq!(settings_data.round_submitter, new_round_submitter);
    assert_eq!(settings_data.min_required_votes, new_min_required_votes);
}

#[tokio::test]
async fn test_create_proposal() {
    let mut program_test = ProgramTest::new(
        "round_loader",
        round_loader::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let proposal_creator = Keypair::new();
    program_test.add_account(
        proposal_creator.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let round_number = 0;

    // Add Relays Accounts
    let mut relays = vec![];
    for _ in 0..100 {
        relays.push(Keypair::new());
    }

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

    let settings_address = get_settings_address();
    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        current_round_number: round_number,
        round_submitter: Pubkey::new_unique(),
        min_required_votes: 1,
        round_ttl: 1209600,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let relay_round_address = get_relay_round_address(round_number);

    let relay_round_data = RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
        round_number,
        round_end: chrono::Utc::now().timestamp() as u32,
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
    };

    let mut relay_round_packed = vec![0; RelayRound::LEN];
    RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    // Create Proposal
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let new_relays = vec![Pubkey::new_unique(); 100];
    let new_round_number = round_number + 1;
    let new_round_end = 1759950990;
    let write_data =
        RelayRoundProposalEventWithLen::new(new_round_number, new_relays.clone(), new_round_end);

    let serialized_write_data = write_data
        .data
        .try_to_vec()
        .expect("serialize proposal event data");

    let proposal_pubkey = get_proposal_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        &serialized_write_data,
    );

    let mut transaction = Transaction::new_with_payer(
        &[create_proposal_ix(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            round_number,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            &serialized_write_data,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Write Proposal
    let chunk_size = 800;

    for (chunk, i) in write_data.try_to_vec().unwrap().chunks(chunk_size).zip(0..) {
        let mut transaction = Transaction::new_with_payer(
            &[write_proposal_ix(
                &proposal_pubkey,
                (i * chunk_size) as u32,
                chunk.to_vec(),
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder], recent_blockhash);

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");
    }

    // Finalize Proposal
    let mut transaction = Transaction::new_with_payer(
        &[finalize_proposal_ix(
            &funder.pubkey(),
            &proposal_pubkey,
            round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_info = banks_client
        .get_account(proposal_pubkey)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");

    assert_eq!(proposal_data.is_initialized, true);
    assert_eq!(proposal_data.account_kind, AccountKind::Proposal);
    assert_eq!(proposal_data.is_executed, false);
    assert_eq!(proposal_data.round_number, round_number);
    assert_eq!(
        proposal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(proposal_data.pda.settings, settings_address);
    assert_eq!(proposal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(proposal_data.pda.event_transaction_lt, event_transaction_lt);

    assert_eq!(proposal_data.signers, vec![Vote::None; relays.len()]);

    assert_eq!(proposal_data.event.data.relays, new_relays);
    assert_eq!(proposal_data.event.data.round_end, new_round_end);

    // Vote for Proposal
    for relay in &relays {
        let mut transaction = Transaction::new_with_payer(
            &[vote_for_proposal_ix(
                &relay.pubkey(),
                &proposal_pubkey,
                round_number,
                Vote::Confirm,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, relay], recent_blockhash);

        let _ = banks_client.process_transaction(transaction).await;
    }

    // Execute Proposal
    let mut transaction = Transaction::new_with_payer(
        &[execute_proposal_ix(
            &funder.pubkey(),
            &proposal_pubkey,
            new_round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_info = banks_client
        .get_account(proposal_pubkey)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");
    assert_eq!(proposal_data.is_executed, true);

    // Check created Relay Round
    let relay_round_address = get_relay_round_address(new_round_number);

    let relay_round_account = banks_client
        .get_account(relay_round_address)
        .await
        .expect("get_account")
        .expect("account");
    let relay_round_data =
        RelayRound::unpack(relay_round_account.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_number, new_round_number);
    assert_eq!(relay_round_data.relays, new_relays);

    // Check Settings
    let settings_account = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");
    let settings_data = Settings::unpack(settings_account.data()).expect("settings unpack");

    assert_eq!(settings_data.current_round_number, new_round_number);
}

#[tokio::test]
async fn test_create_proposal_and_execute_by_admin() {
    let mut program_test = ProgramTest::new(
        "round_loader",
        round_loader::id(),
        processor!(Processor::process),
    );

    // Setup environment
    let proposal_creator = Keypair::new();
    program_test.add_account(
        proposal_creator.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let round_number = 0;

    // Add Relays Accounts
    let mut relays = vec![];
    for _ in 0..100 {
        relays.push(Keypair::new());
    }

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

    // Setup environment
    let round_submitter = Keypair::new();
    program_test.add_account(
        round_submitter.pubkey(),
        Account {
            lamports: 100000000,
            data: vec![],
            owner: solana_program::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let settings_address = get_settings_address();
    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        current_round_number: round_number,
        round_submitter: round_submitter.pubkey(),
        min_required_votes: 1,
        round_ttl: 1209600,
    };

    let mut settings_packed = vec![0; Settings::LEN];
    Settings::pack(settings_account_data, &mut settings_packed).unwrap();
    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: settings_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let relay_round_address = get_relay_round_address(round_number);

    let relay_round_data = RelayRound {
        is_initialized: true,
        account_kind: AccountKind::RelayRound,
        round_number,
        round_end: chrono::Utc::now().timestamp() as u32,
        relays: relays.iter().map(|pair| pair.pubkey()).collect(),
    };

    let mut relay_round_packed = vec![0; RelayRound::LEN];
    RelayRound::pack(relay_round_data, &mut relay_round_packed).unwrap();

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    // Create Proposal
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();

    let new_relays = vec![Pubkey::new_unique(); 100];
    let new_round_number = round_number + 1;
    let new_round_end = 1759950990;
    let write_data =
        RelayRoundProposalEventWithLen::new(new_round_number, new_relays.clone(), new_round_end);

    let serialized_write_data = write_data
        .data
        .try_to_vec()
        .expect("serialize proposal event data");

    let proposal_pubkey = get_proposal_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        &serialized_write_data,
    );

    let mut transaction = Transaction::new_with_payer(
        &[create_proposal_ix(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            round_number,
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            &serialized_write_data,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Write Proposal
    let chunk_size = 800;

    for (chunk, i) in write_data.try_to_vec().unwrap().chunks(chunk_size).zip(0..) {
        let mut transaction = Transaction::new_with_payer(
            &[write_proposal_ix(
                &proposal_pubkey,
                (i * chunk_size) as u32,
                chunk.to_vec(),
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder], recent_blockhash);

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");
    }

    // Finalize Proposal
    let mut transaction = Transaction::new_with_payer(
        &[finalize_proposal_ix(
            &proposal_creator.pubkey(),
            &proposal_pubkey,
            round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_info = banks_client
        .get_account(proposal_pubkey)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");

    assert_eq!(proposal_data.is_initialized, true);
    assert_eq!(proposal_data.account_kind, AccountKind::Proposal);
    assert_eq!(proposal_data.is_executed, false);
    assert_eq!(proposal_data.round_number, round_number);
    assert_eq!(
        proposal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(proposal_data.pda.settings, settings_address);
    assert_eq!(proposal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(proposal_data.pda.event_transaction_lt, event_transaction_lt);

    assert_eq!(proposal_data.signers, vec![Vote::None; relays.len()]);

    assert_eq!(proposal_data.event.data.relays, new_relays);
    assert_eq!(proposal_data.event.data.round_end, new_round_end);

    // Execute Proposal by admin
    let mut transaction = Transaction::new_with_payer(
        &[execute_proposal_by_admin_ix(
            &funder.pubkey(),
            &round_submitter.pubkey(),
            &proposal_pubkey,
            new_round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &round_submitter], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_info = banks_client
        .get_account(proposal_pubkey)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");
    assert_eq!(proposal_data.is_executed, true);

    // Check created Relay Round
    let relay_round_address = get_relay_round_address(new_round_number);

    let relay_round_account = banks_client
        .get_account(relay_round_address)
        .await
        .expect("get_account")
        .expect("account");
    let relay_round_data =
        RelayRound::unpack(relay_round_account.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_number, new_round_number);
    assert_eq!(relay_round_data.relays, new_relays);

    // Check Settings
    let settings_account = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");
    let settings_data = Settings::unpack(settings_account.data()).expect("settings unpack");

    assert_eq!(settings_data.current_round_number, new_round_number);
}

#[tokio::test]
async fn test_close_withdrawal_account() {
    let mut program_test = ProgramTest::new(
        "round_loader",
        round_loader::id(),
        processor!(Processor::process),
    );

    // Setup environment

    // Add Withdrawal Account
    let author = Keypair::new();

    let round_number = 1;
    let event_timestamp = 1650988297;
    let event_transaction_lt = 1650988334;
    let event_configuration = Pubkey::new_unique();
    let settings = get_settings_address();

    let round_num = 1;
    let relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let round_end = 1;

    let event_data = RelayRoundProposalEventWithLen::new(round_num, relays.clone(), round_end)
        .data
        .try_to_vec()
        .unwrap();

    let proposal_address = get_proposal_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        &event_data,
    );

    let proposal_account_data = RelayRoundProposal {
        is_initialized: true,
        account_kind: AccountKind::Proposal,
        is_executed: false,
        author: author.pubkey(),
        round_number,
        event: RelayRoundProposalEventWithLen::new(round_num, relays.clone(), round_end),
        meta: RelayRoundProposalMetaWithLen::new(),
        required_votes: relays.len() as u32,
        signers: relays.iter().map(|_| Vote::None).collect(),
        pda: PDA {
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            settings,
        },
    };

    let mut proposal_packed = vec![0; RelayRoundProposal::LEN];
    RelayRoundProposal::pack(proposal_account_data, &mut proposal_packed).unwrap();
    program_test.add_account(
        proposal_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRoundProposal::LEN)
                + RELAY_REPARATION * 3,
            data: proposal_packed,
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[close_proposal_account_ix(
            &author.pubkey(),
            &proposal_address,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &author], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let proposal_info = banks_client
        .get_account(proposal_address)
        .await
        .expect("get_account");

    assert_eq!(proposal_info, None);
}
