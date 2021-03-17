use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state;
use crate::state::MoneroProof;
use cosmwasm_std::{Api, Binary, HumanAddr, StdError, StdResult, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub prng_seed: Binary,
    pub secret_monero: Snip20,
    pub viewing_key: String,
    pub min_swap_amount: Uint128,
    pub bridge_minter: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Snip20 {
    pub address: HumanAddr,
    // TODO better DS
    pub contract_hash: String,
}

impl Snip20 {
    pub fn from_stored<A: Api>(from: state::Snip20, api: &A) -> StdResult<Self> {
        Ok(Self {
            address: api.human_address(&from.address)?,
            contract_hash: from.contract_hash,
        })
    }

    pub fn to_stored<A: Api>(&self, api: &A) -> StdResult<state::Snip20> {
        Ok(state::Snip20 {
            address: api.canonical_address(&self.address)?,
            contract_hash: self.contract_hash.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SwapDetails {
    pub to_monero_address: String,
    pub from_secret_address: HumanAddr,
    pub amount: Uint128,
    pub nonce: u32,
}

impl SwapDetails {
    pub fn from_stored<A: Api>(from: state::SwapDetails, api: &A) -> StdResult<Self> {
        Ok(Self {
            to_monero_address: from.to_monero_address,
            amount: from.amount,
            from_secret_address: api.human_address(&from.from_secret_address)?,
            nonce: from.nonce,
        })
    }

    pub fn to_stored<A: Api>(self, api: &A) -> StdResult<state::SwapDetails> {
        Ok(state::SwapDetails {
            to_monero_address: self.to_monero_address,
            amount: self.amount,
            from_secret_address: api.canonical_address(&self.from_secret_address)?,
            nonce: self.nonce,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ChangeAdmin {
        address: HumanAddr,
        padding: Option<String>,
    },
    ChangeSecretMoneroContract {
        secret_monero: Snip20,
        padding: Option<String>,
    },
    ChangeViewingKey {
        key: String,
        padding: Option<String>,
    },
    MintSecretMonero {
        proof: MoneroProof,   // Alice's monero proof of payment
        recipient: HumanAddr, // Alice's secret wallet addr
        amount: Uint128,
        padding: Option<String>,
    },
    Receive {
        from: HumanAddr,
        sender: HumanAddr,
        amount: Uint128,
        msg: Binary,
    },
    SetMinters {
        minters: Vec<HumanAddr>,
        padding: Option<String>,
    },
    SetContractStatus {
        level: ContractStatusLevel,
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleResult {
    ChangeAdmin { status: ResponseStatus },
    ChangeSecretMoneroContract { status: ResponseStatus },
    ChangeViewingKey { status: ResponseStatus },
    MintSecretMonero { status: ResponseStatus },
    Receive { status: ResponseStatus, nonce: u32 },
    SetContractStatus { status: ResponseStatus },
    SetMinters { status: ResponseStatus },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatusLevel {
    Running,
    Paused,
}

pub fn status_level_to_u8(status_level: ContractStatusLevel) -> u8 {
    match status_level {
        ContractStatusLevel::Running => 0,
        ContractStatusLevel::Paused => 1,
    }
}

pub fn u8_to_status_level(status_level: u8) -> StdResult<ContractStatusLevel> {
    match status_level {
        0 => Ok(ContractStatusLevel::Running),
        1 => Ok(ContractStatusLevel::Paused),
        _ => Err(StdError::generic_err("Invalid state level")),
    }
}

// Take a Vec<u8> and pad it up to a multiple of `block_size`, using spaces at the end.
pub fn space_pad(block_size: usize, message: &mut Vec<u8>) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}
