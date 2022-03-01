#![cfg(feature = "test-bpf")]

use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::rent::Rent;
use solana_program::{
    bpf_loader_upgradeable, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use round_loader::{
    get_associated_proposal_address, get_associated_relay_round_address,
    get_associated_settings_address, Processor, RelayRound, RelayRoundProposal, Settings,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct WriteData {
    round_ttl: i64,
    relays: Vec<Pubkey>,
}

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
    let round_ttl = 1645086922;

    let mut transaction = Transaction::new_with_payer(
        &[round_loader::initialize(
            &funder.pubkey(),
            &creator.pubkey(),
            round_number,
            round_ttl,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let settings_address = get_associated_settings_address();

    let settings_info = banks_client
        .get_account(settings_address)
        .await
        .expect("get_account")
        .expect("account");

    let settings_data = Settings::unpack(settings_info.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.round_number, round_number);

    let relay_round_address = get_associated_relay_round_address(round_number);

    let relay_round_info = banks_client
        .get_account(relay_round_address)
        .await
        .expect("get_account")
        .expect("account");

    let relay_round_data = RelayRound::unpack(relay_round_info.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_number, round_number);
    assert_eq!(relay_round_data.round_ttl, round_ttl);
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

    let current_round_number = 0;
    let new_round_number = current_round_number + 1;

    let setting_address = get_associated_settings_address();
    let setting_data = Settings {
        is_initialized: true,
        round_number: current_round_number,
    };

    program_test.add_account(
        setting_address,
        Account {
            lamports: Rent::default().minimum_balance(Settings::LEN),
            data: setting_data.try_to_vec().expect("try_to_vec"),
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let relay_round_address = get_associated_relay_round_address(current_round_number);
    let mut relay_round_data = [0u8; RelayRound::LEN];

    let mut relay_round = RelayRound {
        is_initialized: true,
        round_number: current_round_number,
        round_ttl: 1645098009,
        relays: vec![proposal_creator.pubkey()],
    }
    .try_to_vec()
    .unwrap();

    let (left, _) = relay_round_data.split_at_mut(relay_round.len());
    left.copy_from_slice(&mut relay_round);

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_data.to_vec(),
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
    let mut transaction = Transaction::new_with_payer(
        &[round_loader::create_proposal(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            &relay_round_address,
            new_round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    let new_relayer = Keypair::new();

    // Write Proposal
    let write_data = WriteData {
        round_ttl: 1645087790,
        relays: vec![new_relayer.pubkey()],
    };

    let chunk_size = 800;

    for (chunk, i) in write_data.try_to_vec().unwrap().chunks(chunk_size).zip(0..) {
        let mut transaction = Transaction::new_with_payer(
            &[round_loader::write_proposal(
                &proposal_creator.pubkey(),
                new_round_number,
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
        &[round_loader::finalize_proposal(
            &proposal_creator.pubkey(),
            &relay_round_address,
            new_round_number,
        )],
        Some(&proposal_creator.pubkey()),
    );
    transaction.sign(&[&proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check created Proposal
    let proposal_address =
        get_associated_proposal_address(&proposal_creator.pubkey(), new_round_number);

    let proposal_info = banks_client
        .get_account(proposal_address)
        .await
        .expect("get_account")
        .expect("account");

    let proposal_data = RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");

    assert_eq!(proposal_data.is_initialized, true);
    assert_eq!(proposal_data.author, proposal_creator.pubkey());
    assert_eq!(proposal_data.round_number, new_round_number);
    assert_eq!(proposal_data.round_ttl, 1645087790);
    assert_eq!(proposal_data.is_executed, false);
    assert_eq!(proposal_data.voters.len(), 0);
    assert_eq!(proposal_data.required_votes, 1);
    assert_eq!(proposal_data.relays, vec![new_relayer.pubkey()]);

    // Vote for Proposal
    let new_round_address = get_associated_relay_round_address(new_round_number);

    let mut transaction = Transaction::new_with_payer(
        &[round_loader::vote_for_proposal(
            &funder.pubkey(),
            &proposal_creator.pubkey(),
            &proposal_creator.pubkey(),
            &relay_round_address,
            &new_round_address,
            new_round_number,
        )],
        Some(&funder.pubkey()),
    );
    transaction.sign(&[&funder, &proposal_creator], recent_blockhash);

    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");

    // Check Settings
    let settings_account = banks_client
        .get_account(setting_address)
        .await
        .expect("get_account")
        .expect("account");
    let settings_data = Settings::unpack(settings_account.data()).expect("settings unpack");

    assert_eq!(settings_data.is_initialized, true);
    assert_eq!(settings_data.round_number, new_round_number);

    // Check created Relay Round
    let relay_round_account = banks_client
        .get_account(new_round_address)
        .await
        .expect("get_account")
        .expect("account");
    let relay_round_data =
        RelayRound::unpack(relay_round_account.data()).expect("relay round unpack");

    assert_eq!(relay_round_data.is_initialized, true);
    assert_eq!(relay_round_data.round_ttl, 1645087790);
    assert_eq!(relay_round_data.round_number, new_round_number);
    assert_eq!(relay_round_data.relays, vec![new_relayer.pubkey()]);
}
