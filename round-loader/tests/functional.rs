#![cfg(feature = "test-bpf")]

use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::rent::Rent;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use round_loader::{
    get_associated_proposal_address, get_associated_relay_round_address,
    get_associated_settings_address, Processor, RelayRound, RelayRoundProposal, Settings,
    MAX_RELAYS,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct WriteData {
    round_ttl: u32,
    relays: Vec<Pubkey>,
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

    let mut voters = Vec::with_capacity(MAX_RELAYS);
    for _ in 0..MAX_RELAYS - 1 {
        voters.push(Keypair::new());
    }

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
    let relay_round_data = RelayRound {
        is_initialized: true,
        round_number: current_round_number,
        round_ttl: 1645098009,
        relays: {
            let mut relays = vec![proposal_creator.pubkey()];
            relays.extend(
                voters
                    .iter()
                    .map(|x| x.pubkey().clone())
                    .collect::<Vec<Pubkey>>(),
            );
            relays
        },
    };

    program_test.add_account(
        relay_round_address,
        Account {
            lamports: Rent::default().minimum_balance(RelayRound::LEN),
            data: relay_round_data.try_to_vec().expect("try_to_vec"),
            owner: round_loader::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Start Program Test
    let (mut banks_client, funder, recent_blockhash) = program_test.start().await;

    // Fund a balance
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
    assert_eq!(
        proposal_data.required_votes,
        (MAX_RELAYS * 2 / 3 + 1) as u32
    );
    assert_eq!(
        *proposal_data.relays.first().expect("new relay"),
        new_relayer.pubkey()
    );

    // Vote for Proposal
    let new_round_address = get_associated_relay_round_address(new_round_number);

    for voter in voters {
        let mut transaction = Transaction::new_with_payer(
            &[round_loader::vote_for_proposal(
                &funder.pubkey(),
                &proposal_creator.pubkey(),
                &voter.pubkey(),
                &relay_round_address,
                &new_round_address,
                new_round_number,
            )],
            Some(&funder.pubkey()),
        );
        transaction.sign(&[&funder, &voter], recent_blockhash);

        banks_client
            .process_transaction(transaction)
            .await
            .expect("process_transaction");

        let proposal_info = banks_client
            .get_account(proposal_address)
            .await
            .expect("get_account")
            .expect("account");

        let proposal_data =
            RelayRoundProposal::unpack(proposal_info.data()).expect("proposal unpack");
        if proposal_data.is_executed == true {
            break;
        }
    }

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
}
