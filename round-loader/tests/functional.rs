#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::types::Vote;
use std::str::FromStr;

use bridge_utils::state::AccountKind;
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
    let creator = Keypair::new();
    program_test.add_account(
        creator.pubkey(),
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
        upgrade_authority_address: Some(creator.pubkey()),
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

    let round_number = 0;
    let round_end = 1752375045;
    let relays = vec![Pubkey::from_str("2Xzby8BnopnMbCS12YgASrxJoemVFJFgSbSB8pbU1am3").unwrap()];

    let mut transaction = Transaction::new_with_payer(
        &[initialize_ix(
            &funder.pubkey(),
            &creator.pubkey(),
            round_number,
            round_end,
            relays.clone(),
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &creator], recent_blockhash);

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
    assert_eq!(settings_data.round_number, round_number);

    let relay_round_address = get_relay_round_address(round_number);

    let relay_round_info = banks_client
        .get_account(relay_round_address)
        .await
        .expect("get_account")
        .expect("account");

    let relay_round_data = RelayRound::unpack(relay_round_info.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_number, round_number);
    assert_eq!(relay_round_data.round_end, round_end);
    assert_eq!(relay_round_data.relays, relays);
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
    let round_end = 1759950985;

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

    let settings_address = get_settings_address();
    let settings_account_data = Settings {
        is_initialized: true,
        account_kind: AccountKind::Settings,
        round_number,
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
        round_end,
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
        RelayRoundProposalEventWithLen::new(new_round_number, new_relays.clone(), new_round_end)
            .unwrap();

    let serialized_write_data = write_data
        .try_to_vec()
        .expect("serialize proposal event data");

    let proposal_pubkey = get_proposal_address(
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        &serialized_write_data[4..],
    );

    let mut transaction = Transaction::new_with_payer(
        &[create_proposal_ix(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            event_timestamp,
            event_transaction_lt,
            event_configuration,
            &serialized_write_data[4..],
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
                &proposal_creator.pubkey(),
                &proposal_pubkey,
                (i * chunk_size) as u32,
                chunk.to_vec(),
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

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
    assert_eq!(proposal_data.round_number, round_number);
    assert_eq!(
        proposal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(proposal_data.pda.settings, settings_address);
    assert_eq!(proposal_data.pda.event_timestamp, event_timestamp);
    assert_eq!(proposal_data.pda.event_transaction_lt, event_transaction_lt);

    assert_eq!(proposal_data.signers, vec![Vote::None; relays.len()]);

    assert_eq!(
        proposal_data.event.data.relays,
        new_relays
            .iter()
            .map(|relay| relay.to_bytes().to_vec())
            .collect::<Vec<Vec<u8>>>()
    );
    assert_eq!(proposal_data.event.data.round_end, new_round_end);

    assert_eq!(proposal_data.meta.data.is_executed, false);

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

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");
    }

    // Check created Proposal
    let proposal_info = banks_client
        .get_account(proposal_pubkey)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");
    assert_eq!(proposal_data.signers, vec![Vote::Confirm; relays.len()]);

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
    assert_eq!(proposal_data.meta.data.is_executed, true);

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
    assert_eq!(relay_round_data.round_end, new_round_end);
    assert_eq!(relay_round_data.round_number, new_round_number);
    assert_eq!(relay_round_data.relays, new_relays);

    // Check Settings
    let settings_account = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");
    let settings_data = Settings::unpack(settings_account.data()).expect("settings unpack");

    assert_eq!(settings_data.round_number, new_round_number);
}
