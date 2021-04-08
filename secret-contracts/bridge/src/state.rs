use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::convert::TryFrom;

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use crate::viewing_key::ViewingKey;
use secret_toolkit::storage::{AppendStore, AppendStoreMut};

use crate::constants::*;
use crate::msg::{status_level_to_u8, u8_to_status_level, ContractStatusLevel};

fn set_bin_data<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], data: &T) -> StdResult<()> {
    let bin_data =
        bincode2::serialize(&data).map_err(|e| StdError::serialize_err(type_name::<T>(), e))?;

    storage.set(key, &bin_data);
    Ok(())
}

fn get_bin_data<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    let bin_data = storage.get(key);

    match bin_data {
        None => Err(StdError::not_found("Key not found in storage")),
        Some(bin_data) => Ok(bincode2::deserialize::<T>(&bin_data)
            .map_err(|e| StdError::serialize_err(type_name::<T>(), e))?),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SwapDetails {
    pub to_monero_address: String,
    pub from_secret_address: CanonicalAddr,
    pub amount: Uint128,
    pub nonce: u32,
}

pub struct SwapDetailsStore<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> SwapDetailsStore<'a, S> {
    pub fn init(s: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(SWAP_DETAILS_KEY, s),
        }
    }

    pub fn save(&mut self, sd: &mut SwapDetails) -> StdResult<u32> {
        let mut s = AppendStoreMut::attach_or_create(&mut self.storage)?;
        let nonce = s.len();
        sd.nonce = nonce;
        s.push(sd)?;
        Ok(nonce)
    }
}

pub struct ReadonlySwapDetailsStore<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlySwapDetailsStore<'a, S> {
    pub fn init(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(SWAP_DETAILS_KEY, storage),
        }
    }

    pub fn fetch_swap_details(
        &self,
        scrt_addr: CanonicalAddr,
        nonce: u32,
    ) -> StdResult<SwapDetails> {
        let not_found_err = Err(StdError::NotFound {
            kind: std::str::from_utf8(SWAP_DETAILS_KEY).unwrap().to_string(), // no reason to err
            backtrace: None,
        });
        let store = if let Some(result) = AppendStore::<SwapDetails, _>::attach(&self.storage) {
            result?
        } else {
            return not_found_err;
        };

        let res = store.into_iter().find(|swap_details| match swap_details {
            Ok(sd) => sd.nonce == nonce && sd.from_secret_address == scrt_addr,
            Err(_) => false,
        });
        match res {
            Some(swap_details) => swap_details,
            None => not_found_err,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct Constants {
    pub admin: CanonicalAddr,
    pub snip20: Snip20,
    pub viewing_key: String,
    pub prng_seed: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Snip20 {
    pub address: CanonicalAddr,
    // TODO better DS
    pub contract_hash: String,
}

pub struct ConfigStore<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ConfigStore<'a, S> {
    pub fn init(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(CONFIG_KEY, storage),
        }
    }

    pub fn set_constants(&mut self, constants: &Constants) -> StdResult<()> {
        set_bin_data(&mut self.storage, CONSTANTS_KEY, constants)
    }

    pub fn set_min_swap(&mut self, min: u128) {
        self.storage.set(MIN_SWAP_AMOUNT_KEY, &min.to_be_bytes());
    }

    pub fn set_contract_status(&mut self, status: ContractStatusLevel) {
        let status_u8 = status_level_to_u8(status);
        self.storage
            .set(CONTRACT_STATUS_KEY, &status_u8.to_be_bytes());
    }

    pub fn set_minters(&mut self, minters: Vec<CanonicalAddr>) -> StdResult<()> {
        set_bin_data(&mut self.storage, MINTERS_KEY, &minters)
    }
}

pub struct ReadonlyConfigStore<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyConfigStore<'a, S> {
    pub fn init(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(CONFIG_KEY, storage),
        }
    }

    pub fn constants(&self) -> StdResult<Constants> {
        let consts_bytes = self
            .storage
            .get(CONSTANTS_KEY)
            .ok_or_else(|| StdError::generic_err("no constants stored in configuration"))?;
        bincode2::deserialize::<Constants>(&consts_bytes)
            .map_err(|e| StdError::serialize_err(type_name::<Constants>(), e))
    }

    pub fn contract_status(&self) -> ContractStatusLevel {
        let bytes = self
            .storage
            .get(CONTRACT_STATUS_KEY)
            .expect("no contract status stored in config");

        // These unwraps are ok because we know we stored things correctly
        let status = slice_to_u8(&bytes).unwrap();
        u8_to_status_level(status).unwrap()
    }

    pub fn min_swap_amount(&self) -> u128 {
        let supply_bytes = self
            .storage
            .get(MIN_SWAP_AMOUNT_KEY)
            .expect("no min swap amount stored in config");

        // These unwraps are ok because we know we stored things correctly
        slice_to_u128(&supply_bytes).unwrap()
    }

    pub fn minters(&self) -> Vec<CanonicalAddr> {
        get_bin_data(&self.storage, MINTERS_KEY).unwrap()
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct MoneroProof {
    pub tx_id: String,   // corresponds to monero txID
    pub tx_key: String,  // corresponds to monero txKey
    pub address: String, // monero bridge multi-sig wallet
}

pub struct MoneroProofsStore<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> MoneroProofsStore<'a, S> {
    pub fn init(s: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(MONERO_PROOFS_KEY, s),
        }
    }

    pub fn save(&mut self, mp: &mut MoneroProof) -> StdResult<()> {
        AppendStoreMut::attach_or_create(&mut self.storage)?.push(mp)
    }
}

pub struct ReadonlyMoneroProofsStore<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyMoneroProofsStore<'a, S> {
    pub fn init(s: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(MONERO_PROOFS_KEY, s),
        }
    }

    pub fn fetch_by_tx_id(&self, tx_id: &str) -> StdResult<MoneroProof> {
        let not_found_err = Err(StdError::NotFound {
            kind: std::str::from_utf8(MONERO_PROOFS_KEY).unwrap().to_string(), // no reason to err
            backtrace: None,
        });
        let store = if let Some(result) = AppendStore::<MoneroProof, _>::attach(&self.storage) {
            result?
        } else {
            return not_found_err;
        };

        let res = store.into_iter().find(|proof| match proof {
            Ok(p) => p.tx_id == tx_id,
            Err(_) => false,
        });
        match res {
            Some(proof) => proof,
            None => not_found_err,
        }
    }
}

// Viewing Keys

pub fn set_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
    let mut balance_store = PrefixedStorage::new(VK_KEY, store);
    balance_store.set(owner.as_slice(), &key.to_hashed());
}

pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
    let balance_store = ReadonlyPrefixedStorage::new(VK_KEY, store);
    balance_store.get(owner.as_slice())
}

// Helpers

/// Converts 1 byte value into u8
/// Errors if data found that is not 1 byte
fn slice_to_u8(data: &[u8]) -> StdResult<u8> {
    if data.len() == 1 {
        Ok(data[0])
    } else {
        Err(StdError::generic_err(
            "Corrupted data found. 1 byte expected.",
        ))
    }
}

/// Converts 16 bytes value into u128
/// Errors if data found that is not 16 bytes
fn slice_to_u128(data: &[u8]) -> StdResult<u128> {
    match <[u8; 16]>::try_from(data) {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 16 byte expected.",
        )),
    }
}
