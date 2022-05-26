use std::str::FromStr;

use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use bridge_utils::helper::get_programdata_address;

use bridge_utils::state::*;
use bridge_utils::types::*;

use crate::*;

#[wasm_bindgen(js_name = "initialize")]
pub fn initialize_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    genesis_round_number: u32,
    round_submitter: String,
    min_required_votes: u32,
) -> Result<JsValue, JsValue> {
    let program_id = &id();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;
    let round_submitter = Pubkey::from_str(round_submitter.as_str()).handle_error()?;

    let setting_pubkey = get_associated_settings_address(program_id);
    let program_data_pubkey = get_programdata_address(program_id);

    let data = RoundLoaderInstruction::Initialize {
        genesis_round_number,
        round_submitter,
        min_required_votes,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "updateSettings")]
pub fn update_settings_ix(
    author_pubkey: String,
    current_round_number: Option<u32>,
    round_submitter: Option<String>,
    min_required_votes: Option<u32>,
) -> Result<JsValue, JsValue> {
    let program_id = &id();

    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;

    let setting_pubkey = get_associated_settings_address(program_id);
    let program_data_pubkey = get_programdata_address(program_id);

    let round_submitter = round_submitter
        .map(|value| Pubkey::from_str(value.as_str()))
        .transpose()
        .handle_error()?;

    let data = RoundLoaderInstruction::UpdateSettings {
        current_round_number,
        round_submitter,
        min_required_votes,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "createRelayRound")]
pub fn create_relay_round_ix(
    funder_pubkey: String,
    creator_pubkey: String,
    round_number: u32,
    relays: JsValue,
) -> Result<JsValue, JsValue> {
    let program_id = &id();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let creator_pubkey = Pubkey::from_str(creator_pubkey.as_str()).handle_error()?;

    let setting_pubkey = get_associated_settings_address(program_id);
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let relays: Vec<String> = relays.into_serde().handle_error()?;
    let relays = relays
        .into_iter()
        .map(|x| Pubkey::from_str(x.as_str()).unwrap())
        .collect();

    let data = RoundLoaderInstruction::CreateRelayRound {
        round_number,
        relays,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(creator_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "execute")]
pub fn execute_ix(
    funder_pubkey: String,
    proposal_pubkey: String,
    round_number: u32,
) -> Result<JsValue, JsValue> {
    let program_id = &id();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let proposal_pubkey = Pubkey::from_str(proposal_pubkey.as_str()).handle_error()?;

    let settings_pubkey = get_associated_settings_address(program_id);
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::ExecuteProposal
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let settings = Settings::unpack(&data).handle_error()?;

    let s = WasmSettings {
        is_initialized: settings.is_initialized,
        current_round_number: settings.current_round_number,
        round_submitter: settings.round_submitter,
        min_required_votes: settings.min_required_votes,
    };

    return JsValue::from_serde(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackRelayRound")]
pub fn unpack_relay_round(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let relay_round = RelayRound::unpack(&data).handle_error()?;

    let rr = WasmRelayRound {
        is_initialized: relay_round.is_initialized,
        round_number: relay_round.round_number,
        relays: relay_round.relays,
    };

    return JsValue::from_serde(&rr).handle_error();
}

#[wasm_bindgen(js_name = "unpackRelayRoundProposal")]
pub fn unpack_relay_round_proposal(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let relay_round_proposal = RelayRoundProposal::unpack(&data).handle_error()?;

    let rrp = WasmRelayRoundProposal {
        is_initialized: relay_round_proposal.is_initialized,
        round_number: relay_round_proposal.round_number,
        required_votes: relay_round_proposal.required_votes,
        pda: relay_round_proposal.pda,
        event: relay_round_proposal.event,
        meta: relay_round_proposal.meta,
        signers: relay_round_proposal.signers,
    };

    return JsValue::from_serde(&rrp).handle_error();
}

#[derive(Serialize, Deserialize)]
pub struct WasmSettings {
    pub is_initialized: bool,
    pub current_round_number: u32,
    pub round_submitter: Pubkey,
    pub min_required_votes: u32,
}

#[derive(Serialize, Deserialize)]
pub struct WasmRelayRound {
    pub is_initialized: bool,
    pub round_number: u32,
    pub relays: Vec<Pubkey>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmRelayRoundProposal {
    pub is_initialized: bool,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: RelayRoundProposalEventWithLen,
    pub meta: RelayRoundProposalMetaWithLen,
    pub signers: Vec<Vote>,
}

impl<T, E> HandleError for Result<T, E>
where
    E: ToString,
{
    type Output = T;

    fn handle_error(self) -> Result<Self::Output, JsValue> {
        self.map_err(|e| {
            let error = e.to_string();
            js_sys::Error::new(&error).unchecked_into()
        })
    }
}

pub trait HandleError {
    type Output;

    fn handle_error(self) -> Result<Self::Output, JsValue>;
}
