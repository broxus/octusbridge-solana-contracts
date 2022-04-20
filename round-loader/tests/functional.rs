#![cfg(feature = "test-bpf")]

use borsh::BorshSerialize;
use bridge_utils::types::Vote;
use rand::Rng;

use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::rent::Rent;
use solana_program::{
    bpf_loader_upgradeable, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
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
    let round_end = 1645086922;

    let mut transaction = Transaction::new_with_payer(
        &[round_loader::initialize_ix(
            &funder.pubkey(),
            &creator.pubkey(),
            round_number,
            round_end,
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
    assert_eq!(relay_round_data.relays, vec![creator.pubkey()]);
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

    let round_number = 0;
    let round_end = 1759950985;

    let relays = vec![Keypair::new(), Keypair::new(), Keypair::new()];

    let settings_address = get_settings_address();
    let setting_data = Settings {
        is_initialized: true,
        round_number,
    };

    program_test.add_account(
        settings_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: setting_data.try_to_vec().expect("try_to_vec"),
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let relay_round_address = get_relay_round_address(round_number);

    let relay_round_data = RelayRound {
        is_initialized: true,
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

    // TODO: fix this workaround
    // Fund a balance of creator since test stucks
    let require_balance = Rent::default().minimum_balance(RelayRoundProposal::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::transfer(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            require_balance,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    assert_eq!(
        banks_client
            .get_balance(proposal_creator.pubkey())
            .await
            .expect("get_balance"),
        require_balance
    );

    // Create Proposal
    let proposal_seed = rand::thread_rng().gen::<u64>();

    let mut transaction = Transaction::new_with_payer(
        &[round_loader::create_proposal_ix(
            &funder.pubkey(),
            proposal_seed,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Write Proposal
    let new_relays = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let new_round_number = round_number + 1;
    let new_round_end = 1759950990;
    let write_data =
        RelayRoundProposalEventWithLen::new(new_round_number, new_relays.clone(), new_round_end)
            .unwrap();

    let chunk_size = 800;

    for (chunk, i) in write_data.try_to_vec().unwrap().chunks(chunk_size).zip(0..) {
        let mut transaction = Transaction::new_with_payer(
            &[round_loader::write_proposal_ix(
                proposal_seed,
                (i * chunk_size) as u32,
                chunk.to_vec(),
            )],
            Some(&proposal_creator.pubkey()),
        );
        transaction.sign(&[&proposal_creator], recent_blockhash);

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");
    }

    // Finalize Proposal
    let mut transaction = Transaction::new_with_payer(
        &[round_loader::finalize_proposal_ix(
            &proposal_creator.pubkey(),
            proposal_seed,
            round_number,
        )],
        Some(&proposal_creator.pubkey()),
    );
    transaction.sign(&[&proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_address = get_proposal_address(proposal_seed, &settings_address);

    let proposal_info = banks_client
        .get_account(proposal_address)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");

    assert_eq!(proposal_data.is_initialized, true);
    assert_eq!(proposal_data.round_number, round_number);
    assert_eq!(proposal_data.signers, vec![Vote::None; relays.len()]);
    assert_eq!(
        proposal_data.required_votes,
        (relays.len() * 2 / 3 + 1) as u32
    );

    assert_eq!(proposal_data.event.data.relays, new_relays);
    assert_eq!(proposal_data.event.data.round_end, new_round_end);

    assert_eq!(proposal_data.meta.data.is_executed, false);
    assert_eq!(proposal_data.meta.data.author, proposal_creator.pubkey());

    // Vote for Proposal
    for relay in &relays {
        let mut transaction = Transaction::new_with_payer(
            &[round_loader::vote_for_proposal_ix(
                &relay.pubkey(),
                proposal_seed,
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
        .get_account(proposal_address)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");
    assert_eq!(proposal_data.signers, vec![Vote::Confirm; relays.len()]);

    // Execute Proposal
    let mut transaction = Transaction::new_with_payer(
        &[round_loader::execute_proposal_ix(
            &funder.pubkey(),
            proposal_seed,
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
        .get_account(proposal_address)
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
