#![cfg(feature = "test-bpf")]

use solana_program::{bpf_loader_upgradeable, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use round_loader::Processor;

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "round_loader",
        round_loader::id(),
        processor!(Processor::process),
    )
}

#[tokio::test]
async fn test_init_round_loader() {
    let (mut banks_client, funder, recent_blockhash) = program_test().start().await;

    let mut transaction = Transaction::new_with_payer(
        &[round_loader::initialize(&funder.pubkey(), 0, 1645086922)],
        Some(&funder.pubkey()),
    );

    transaction.sign(&[&funder], recent_blockhash);
    banks_client
        .process_transaction(transaction)
        .await
        .expect("process_transaction");
}
