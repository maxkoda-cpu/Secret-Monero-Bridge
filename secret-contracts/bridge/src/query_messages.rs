use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};

use crate::msg::{ContractStatusLevel, Snip20};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    SecretMoneroBalance { this_address: HumanAddr },
    Config {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryResponse {
    Config {
        admin: HumanAddr,
        minters: Vec<HumanAddr>,
        min_swap: Uint128,
        secret_monero: Snip20,
        status: ContractStatusLevel,
    },
    SecretMoneroBalance {
        balance: Uint128,
    },
}
