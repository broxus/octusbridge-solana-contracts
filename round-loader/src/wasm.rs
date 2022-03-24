use std::str::FromStr;

use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::{
    get_associated_relay_round_address, get_associated_settings_address, get_program_data_address,
    id, RoundLoaderInstruction,
};

#[wasm_bindgen(js_name = "initialize")]
pub fn initialize_ix(
    funder_pubkey: String,
    creator_pubkey: String,
    round: u32,
    round_ttl: i64,
) -> JsValue {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).unwrap();
    let creator_pubkey = Pubkey::from_str(creator_pubkey.as_str()).unwrap();

    let setting_pubkey = get_associated_settings_address();
    let program_data_pubkey = get_program_data_address();
    let relay_round_pubkey = get_associated_relay_round_address(round);

    let data = RoundLoaderInstruction::Initialize { round, round_ttl }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(creator_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> JsValue {
    let settings = crate::Settings::unpack(&data).unwrap();

    let s = Settings {
        is_initialized: settings.is_initialized,
        round_number: settings.round_number,
    };

    return JsValue::from_serde(&s).unwrap();
}

#[wasm_bindgen(js_name = "unpackRelayRound")]
pub fn unpack_relay_round(data: Vec<u8>) -> JsValue {
    let relay_round = crate::RelayRound::unpack(&data).unwrap();

    let rr = RelayRound {
        is_initialized: relay_round.is_initialized,
        round_number: relay_round.round_number,
        round_ttl: relay_round.round_ttl,
        relays: relay_round.relays,
    };

    return JsValue::from_serde(&rr).unwrap();
}

#[wasm_bindgen(js_name = "unpackRelayRoundProposal")]
pub fn unpack_relay_round_proposal(data: Vec<u8>) -> JsValue {
    let relay_round_proposal = crate::RelayRoundProposal::unpack(&data).unwrap();

    let rrp = RelayRoundProposal {
        is_initialized: relay_round_proposal.is_initialized,
        author: relay_round_proposal.author,
        round_number: relay_round_proposal.round_number,
        required_votes: relay_round_proposal.required_votes,
        is_executed: relay_round_proposal.is_executed,
        round_ttl: relay_round_proposal.round_ttl,
        relays: relay_round_proposal.relays,
        voters: relay_round_proposal.voters,
    };

    return JsValue::from_serde(&rrp).unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub is_initialized: bool,
    pub round_number: u32,
}

#[derive(Serialize, Deserialize)]
pub struct RelayRound {
    pub is_initialized: bool,
    pub round_number: u32,
    pub round_ttl: i64,
    pub relays: Vec<Pubkey>,
}

#[derive(Serialize, Deserialize)]
pub struct RelayRoundProposal {
    pub is_initialized: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub is_executed: bool,
    pub round_ttl: i64,
    pub relays: Vec<Pubkey>,
    pub voters: Vec<Pubkey>,
}
