use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse,
    HumanAddr, InitResponse, Querier, QueryResult, StdError, StdResult, Storage, Uint128,
};

use secret_toolkit::crypto::sha_256;
use secret_toolkit::snip20;

use crate::constants::*;
use crate::msg::ResponseStatus::Success;
use crate::msg::{
    space_pad, ContractStatusLevel, HandleMsg, HandleResult, InitMsg, Snip20, SwapDetails,
};
use crate::query_messages::{QueryMsg, QueryResponse};
use crate::state::{
    read_viewing_key, set_viewing_key, ConfigStore, Constants, MoneroProof, MoneroProofsStore,
    ReadonlyConfigStore, ReadonlyMoneroProofsStore, ReadonlySwapDetailsStore, SwapDetailsStore,
};
use crate::token_msg::TokenMsg;
use crate::viewing_key::ViewingKey;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // Initialize config state
    let mut cfg_store = ConfigStore::init(&mut deps.storage);
    cfg_store.set_constants(&Constants {
        admin: deps.api.canonical_address(&env.message.sender.clone())?,
        snip20: msg.secret_monero.to_stored(&deps.api)?,
        viewing_key: msg.viewing_key.clone(),
        prng_seed: sha_256(&msg.prng_seed.0).to_vec(),
    })?;
    cfg_store.set_minters(vec![deps.api.canonical_address(&msg.bridge_minter)?])?;
    cfg_store.set_contract_status(ContractStatusLevel::Running);
    cfg_store.set_min_swap(msg.min_swap_amount.u128());

    // Register sXMR token, set vk
    let messages = vec![
        snip20::register_receive_msg(
            env.contract_code_hash,
            None,
            1, // This is public data, no need to pad
            msg.secret_monero.contract_hash.clone(),
            msg.secret_monero.address.clone(),
        )?,
        snip20::set_viewing_key_msg(
            msg.viewing_key,
            None,
            BLOCK_SIZE, // This is private data, need to pad
            msg.secret_monero.contract_hash,
            msg.secret_monero.address,
        )?,
    ];

    Ok(InitResponse {
        messages,
        log: vec![],
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::Config {} => query_config(&deps),
        QueryMsg::SecretMoneroBalance { this_address } => query_sxmr_balance(&deps, this_address),
        _ => authenticated_queries(deps, msg),
    }
}

pub fn authenticated_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    match msg {
        QueryMsg::SwapDetails {
            address,
            viewing_key,
            nonce,
        } => query_swap_details(&deps, address, viewing_key, nonce),
        _ => panic!("this query type does not require authentication"),
    }
}

fn query_swap_details<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
    viewing_key: String,
    nonce: u32,
) -> QueryResult {
    let addr = deps.api.canonical_address(&address)?;

    match auth_vk_access(&deps.storage, addr.clone(), viewing_key) {
        Some(err) => to_binary(&err),
        None => {
            let store = ReadonlySwapDetailsStore::init(&deps.storage);
            let sd = store.fetch_swap_details(addr, nonce)?;

            to_binary(&QueryResponse::SwapDetails {
                to_monero_address: sd.to_monero_address,
                from_secret_address: deps.api.human_address(&sd.from_secret_address)?,
                amount: sd.amount,
            })
        }
    }
}

fn auth_vk_access<S: Storage>(
    store: &S,
    address: CanonicalAddr,
    view_key: String,
) -> Option<QueryResponse> {
    let vk = ViewingKey(view_key);
    let expected_key = read_viewing_key(store, &address);

    if expected_key.is_none() {
        return Some(QueryResponse::ViewingKeyError {
            msg: "viewing key for this address is not set".to_string(),
        });
    }
    if !vk.is_valid(expected_key.unwrap().as_slice()) {
        return Some(QueryResponse::ViewingKeyError {
            msg: "incorrect viewing key".to_string(),
        });
    }

    None
}

fn query_config<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let store = ReadonlyConfigStore::init(&deps.storage);
    let consts = store.constants()?;

    to_binary(&QueryResponse::Config {
        admin: deps.api.human_address(&consts.admin)?,
        minters: store
            .minters()
            .iter()
            .map(|addr| deps.api.human_address(addr).unwrap())
            .collect(),
        min_swap: Uint128(store.min_swap_amount()),
        secret_monero: Snip20::from_stored(consts.snip20, &deps.api)?,
        status: store.contract_status(),
    })
}

fn query_sxmr_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    this_address: HumanAddr,
) -> QueryResult {
    let consts = ReadonlyConfigStore::init(&deps.storage).constants()?;
    let sxmr_contract = Snip20::from_stored(consts.snip20, &deps.api)?;
    let resp = snip20::balance_query(
        &deps.querier,
        this_address,
        consts.viewing_key,
        BLOCK_SIZE,
        sxmr_contract.contract_hash,
        sxmr_contract.address,
    )?;

    to_binary(&QueryResponse::SecretMoneroBalance {
        balance: resp.amount,
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    // check contract status is in valid state
    let contract_status = ReadonlyConfigStore::init(&deps.storage).contract_status();
    if contract_status == ContractStatusLevel::Paused {
        let response = match msg {
            HandleMsg::SetContractStatus { level, .. } => set_contract_status(deps, env, level),
            _ => Err(StdError::generic_err(
                "This contract is stopped and this action is not allowed",
            )),
        };
        return pad_response(response);
    }

    match msg {
        HandleMsg::ChangeAdmin { address, .. } => change_admin(deps, env, address),
        HandleMsg::ChangeSecretMoneroContract { secret_monero, .. } => {
            change_sxmr_contract(deps, env, secret_monero)
        }
        HandleMsg::SetViewingKey { key, .. } => set_vk(deps, env, key),
        HandleMsg::MintSecretMonero {
            proof,
            recipient,
            amount,
            ..
        } => mint_sxmr(deps, env, amount, proof, recipient),
        HandleMsg::Receive {
            amount, msg, from, ..
        } => burn_sxmr(deps, env, from, amount, msg),
        HandleMsg::SetContractStatus { level, .. } => set_contract_status(deps, env, level),
        HandleMsg::SetMinters { minters, .. } => set_minters(deps, env, minters),
    }
}

fn set_minters<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_minters: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&deps.storage);
    let sender = &deps.api.canonical_address(&env.message.sender)?;
    auth_admin_access(&store, sender)?;

    let new_minters = new_minters
        .iter()
        .map(|m| deps.api.canonical_address(m).unwrap())
        .collect();
    let mut store = ConfigStore::init(&mut deps.storage);
    store.set_minters(new_minters)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::SetMinters { status: Success })?),
    })
}

fn pad_response(response: StdResult<HandleResponse>) -> StdResult<HandleResponse> {
    response.map(|mut response| {
        response.data = response.data.map(|mut data| {
            space_pad(BLOCK_SIZE, &mut data.0);
            data
        });
        response
    })
}

fn change_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    address: HumanAddr,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&mut deps.storage);
    let sender = &deps.api.canonical_address(&env.message.sender)?;
    auth_admin_access(&store, sender)?;

    let mut consts = store.constants()?;
    consts.admin = deps.api.canonical_address(&address)?;
    let mut store = ConfigStore::init(&mut deps.storage);
    store.set_constants(&consts)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::ChangeAdmin { status: Success })?),
    })
}

fn change_sxmr_contract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_sxmr_contract: Snip20,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&mut deps.storage);
    let sender = &deps.api.canonical_address(&env.message.sender)?;
    auth_admin_access(&store, sender)?;

    let mut consts = store.constants()?;
    let old_contract = Snip20::from_stored(consts.snip20, &deps.api)?;
    consts.snip20 = new_sxmr_contract.to_stored(&deps.api)?;
    let mut store = ConfigStore::init(&mut deps.storage);
    store.set_constants(&consts)?;

    // de-register old sXMR token address, and register receive with the new
    let messages = vec![
        snip20::register_receive_msg(
            env.contract_code_hash,
            None,
            1, // This is public data, no need to pad
            new_sxmr_contract.contract_hash.clone(),
            new_sxmr_contract.address.clone(),
        )?,
        snip20::set_viewing_key_msg(
            consts.viewing_key,
            None,
            BLOCK_SIZE, // This is private data, need to pad
            new_sxmr_contract.contract_hash,
            new_sxmr_contract.address,
        )?,
        TokenMsg::DeRegisterReceive { padding: None }
            .to_cosmos_msg(old_contract.address, old_contract.contract_hash)?,
    ];
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleResult::ChangeSecretMoneroContract {
            status: Success,
        })?),
    })
}

fn set_vk<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    vk: String,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&mut deps.storage);
    if is_admin(&store, &deps.api.canonical_address(&env.message.sender)?)? {
        return set_contract_vk(deps, vk);
    }

    let vk = ViewingKey(vk);
    let message_sender = deps.api.canonical_address(&env.message.sender)?;
    set_viewing_key(&mut deps.storage, &message_sender, &vk);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::SetViewingKey { status: Success })?),
    })
}

fn set_contract_vk<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    vk: String,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&mut deps.storage);
    let sxmr = Snip20::from_stored(store.constants()?.snip20, &deps.api)?;
    let messages = vec![snip20::set_viewing_key_msg(
        vk,
        None,
        BLOCK_SIZE, // This is private data, need to pad
        sxmr.contract_hash,
        sxmr.address,
    )?];
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleResult::SetViewingKey { status: Success })?),
    })
}

fn auth_admin_access<S: Storage>(
    store: &ReadonlyConfigStore<S>,
    account: &CanonicalAddr,
) -> StdResult<()> {
    if !is_admin(store, account)? {
        return Err(StdError::generic_err(
            "This is an admin command. Admin commands can only be run from admin address",
        ));
    }

    Ok(())
}

fn is_admin<S: Storage>(
    store: &ReadonlyConfigStore<S>,
    account: &CanonicalAddr,
) -> StdResult<bool> {
    let consts = store.constants()?;
    if &consts.admin != account {
        return Ok(false);
    }

    Ok(true)
}

fn set_contract_status<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    status_level: ContractStatusLevel,
) -> StdResult<HandleResponse> {
    let store = ReadonlyConfigStore::init(&mut deps.storage);
    auth_admin_access(&store, &deps.api.canonical_address(&env.message.sender)?)?;

    let mut store = ConfigStore::init(&mut deps.storage);
    store.set_contract_status(status_level);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::SetContractStatus {
            status: Success,
        })?),
    })
}

fn is_valid_proof<S: Storage>(tx_id: &str, store: ReadonlyMoneroProofsStore<S>) -> bool {
    // proofs can only be stored once,
    // otherwise users could infinitely mint!
    match store.fetch_by_tx_id(tx_id) {
        Ok(_) => false,
        Err(err) => match err {
            StdError::NotFound { .. } => true,
            _ => panic!("internal err while validating proof"), // this should never happen tbh
        },
    }
}

fn auth_mint<S: Storage>(s: &ReadonlyConfigStore<S>, sender: &CanonicalAddr) -> StdResult<()> {
    if !s.minters().contains(sender) {
        return Err(StdError::generic_err("user is unauthorized to mint"));
    }

    Ok(())
}

fn mint_sxmr<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
    mut proof: MoneroProof,
    recipient: HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg_store = ReadonlyConfigStore::init(&deps.storage);
    if let Err(err) = auth_mint(
        &cfg_store,
        &deps.api.canonical_address(&env.message.sender)?,
    ) {
        return Err(err);
    }

    // send sXMR mint message to token contract
    let sxmr = Snip20::from_stored(cfg_store.constants()?.snip20, &deps.api)?;

    let mp_store = ReadonlyMoneroProofsStore::init(&mut deps.storage);
    if !is_valid_proof(&proof.tx_id, mp_store) {
        return Err(StdError::generic_err("invalid monero proof"));
    }

    let mut mp_store = MoneroProofsStore::init(&mut deps.storage);
    mp_store.save(&mut proof)?;

    // mint sXMR and send to secret_address
    let mint_msg = TokenMsg::Mint {
        amount,
        recipient,
        padding: None, // TODO pad?
        memo: None,
    };
    Ok(HandleResponse {
        messages: vec![mint_msg.to_cosmos_msg(sxmr.address, sxmr.contract_hash)?],
        log: vec![],
        data: Some(to_binary(&HandleResult::MintSecretMonero {
            status: Success,
        })?),
    })
}

fn auth_burn<S: Storage>(s: &ReadonlyConfigStore<S>, sender: CanonicalAddr) -> StdResult<()> {
    // make sure the sender is the sXMR contract
    if sender != s.constants()?.snip20.address {
        return Err(StdError::generic_err(
            "only the sXMR contract can burn tokens",
        ));
    }

    Ok(())
}

fn burn_sxmr<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    amount: Uint128,
    msg: Binary,
) -> StdResult<HandleResponse> {
    let cfg_store = ReadonlyConfigStore::init(&deps.storage);
    if let Err(err) = auth_burn(&cfg_store, deps.api.canonical_address(&env.message.sender)?) {
        return Err(err);
    }

    if amount.u128() < cfg_store.min_swap_amount() {
        return Err(StdError::generic_err(format!(
            "Cannot swap amount under minimum of: {}",
            cfg_store.min_swap_amount()
        )));
    }

    let mut sd: SwapDetails = from_binary(&msg)?;
    sd.from_secret_address = from;
    // TODO validate that destination is a valid Monero address
    // validate_address(&sd.to_monero_address)?;

    // send sXMR burn message to token contract
    let sxmr = Snip20::from_stored(cfg_store.constants()?.snip20, &deps.api)?;
    // store the swap details
    let mut sd_store = SwapDetailsStore::init(&mut deps.storage);
    let nonce = sd_store.save(&mut sd.to_stored(&deps.api)?)?;

    // create sXMR burn message
    let burn = TokenMsg::Burn {
        amount,
        memo: None,
        padding: None, // TODO add padding?
    };
    Ok(HandleResponse {
        messages: vec![burn.to_cosmos_msg(sxmr.address, sxmr.contract_hash)?],
        log: vec![log("tx_id", nonce)],
        data: Some(to_binary(&HandleResult::Receive {
            status: Success,
            nonce,
        })?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ResponseStatus;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};

    fn init_helper(
        init_msg: InitMsg,
        admin_addr: String,
    ) -> Extern<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env(admin_addr, &[]);

        let result = init(&mut deps, env, init_msg);
        assert!(result.is_ok(), "Init failed: {}", result.err().unwrap());

        deps
    }

    #[test]
    fn test_init_sanity() {
        let init_msg = InitMsg {
            prng_seed: Binary::from("seed".as_bytes()),
            secret_monero: Snip20 {
                address: HumanAddr("tokenAddr".to_string()),
                contract_hash: "tokenHash".to_string(),
            },
            viewing_key: "vk".to_string(),
            min_swap_amount: Uint128(10000),
            bridge_minter: HumanAddr("bridgeMinter".to_string()),
        };
        let admin = "segfaultdoc";
        let deps = init_helper(init_msg.clone(), admin.to_string());
        let store = ReadonlyConfigStore::init(&deps.storage);
        let constants = store.constants().unwrap();

        assert_eq!(
            deps.api.human_address(&constants.admin).unwrap(),
            HumanAddr(admin.to_string())
        );
        assert_eq!(
            constants.snip20.contract_hash,
            init_msg.secret_monero.contract_hash
        );
        assert_eq!(
            constants.snip20.address,
            deps.api
                .canonical_address(&init_msg.secret_monero.address)
                .unwrap()
        );
        assert_eq!(constants.viewing_key, init_msg.viewing_key);
        assert_eq!(constants.prng_seed, sha_256(&init_msg.prng_seed.0).to_vec());
        assert_eq!(store.contract_status(), ContractStatusLevel::Running);
        assert_eq!(store.min_swap_amount(), init_msg.min_swap_amount.u128());
        assert_eq!(store.minters().len(), 1);
        assert_eq!(
            deps.api
                .human_address(&store.minters().get(0).unwrap())
                .unwrap(),
            init_msg.bridge_minter
        )
    }

    #[test]
    fn test_query_config() {
        let init_msg = InitMsg {
            prng_seed: Binary::from("seed".as_bytes()),
            secret_monero: Snip20 {
                address: HumanAddr("tokenAddr".to_string()),
                contract_hash: "tokenHash".to_string(),
            },
            viewing_key: "vk".to_string(),
            min_swap_amount: Uint128(10000),
            bridge_minter: HumanAddr("bridgeMinter".to_string()),
        };
        let admin_str = "segfaultdoc";
        let deps = init_helper(init_msg.clone(), admin_str.to_string());

        let query_msg = QueryMsg::Config {};
        let query_result = query(&deps, query_msg);
        assert!(
            query_result.is_ok(),
            "Query failed: {}",
            query_result.err().unwrap()
        );
        let resp: QueryResponse = from_binary(&query_result.unwrap()).unwrap();
        match resp {
            QueryResponse::Config {
                admin,
                minters,
                min_swap,
                secret_monero,
                status,
            } => {
                assert_eq!(admin, HumanAddr(admin_str.to_string()));
                assert_eq!(minters, vec![init_msg.bridge_minter]);
                assert_eq!(min_swap, init_msg.min_swap_amount);
                assert_eq!(secret_monero.address, init_msg.secret_monero.address);
                assert_eq!(
                    secret_monero.contract_hash,
                    init_msg.secret_monero.contract_hash
                );
                assert_eq!(status, ContractStatusLevel::Running)
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn test_query_swap_details() {
        // SETUP
        let init_msg = InitMsg {
            prng_seed: Binary::from("seed".as_bytes()),
            secret_monero: Snip20 {
                address: HumanAddr("tokenAddr".to_string()),
                contract_hash: "tokenHash".to_string(),
            },
            viewing_key: "vk".to_string(),
            min_swap_amount: Uint128(10000),
            bridge_minter: HumanAddr("bridgeMinter".to_string()),
        };
        let mut deps = init_helper(init_msg.clone(), "admin".to_string());
        let user = "segfaultdoc";
        let vk = "my_vk";
        // set vk for user
        let set_vk_msg = HandleMsg::SetViewingKey {
            key: vk.to_string(),
            padding: None,
        };
        let result = handle(&mut deps, mock_env(user.to_string(), &[]), set_vk_msg);
        assert!(
            result.is_ok(),
            "SetViewingKey Failed: {}",
            result.err().unwrap()
        );
        // save SwapDetails
        let mut sd_store = SwapDetailsStore::init(&mut deps.storage);
        let to_monero_address = "some_monero_addr";
        let amount = 10000000;
        let nonce = sd_store
            .save(
                &mut SwapDetails {
                    to_monero_address: to_monero_address.to_string(),
                    from_secret_address: HumanAddr(user.to_string()),
                    amount: Uint128(amount),
                    nonce: 99,
                }
                .to_stored(&deps.api)
                .unwrap(),
            )
            .unwrap();
        assert_eq!(nonce, 0);

        // TEST
        let query_msg = QueryMsg::SwapDetails {
            address: HumanAddr(user.to_string()),
            viewing_key: vk.to_string(),
            nonce: nonce,
        };
        let query_result = query(&deps, query_msg);
        assert!(
            query_result.is_ok(),
            "Query failed: {}",
            query_result.err().unwrap()
        );
        let resp: QueryResponse = from_binary(&query_result.unwrap()).unwrap();
        match resp {
            QueryResponse::SwapDetails {
                to_monero_address: actual_xmr_address,
                from_secret_address,
                amount: actual_amount,
            } => {
                assert_eq!(actual_xmr_address, to_monero_address);
                assert_eq!(from_secret_address, HumanAddr(user.to_string()));
                assert_eq!(actual_amount, Uint128(amount));
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn test_handle_mint_sxmr() {
        let bridge_minter = "bridgeMinter";
        let init_msg = InitMsg {
            prng_seed: Binary::from("seed".as_bytes()),
            secret_monero: Snip20 {
                address: HumanAddr("tokenAddr".to_string()),
                contract_hash: "tokenHash".to_string(),
            },
            viewing_key: "vk".to_string(),
            min_swap_amount: Uint128(10000),
            bridge_minter: HumanAddr(bridge_minter.to_string()),
        };
        let admin_str = "segfaultdoc";
        let mut deps = init_helper(init_msg.clone(), admin_str.to_string());

        let mint_msg = HandleMsg::MintSecretMonero {
            proof: MoneroProof {
                tx_id: "moneroTxID".to_string(),
                tx_key: "moneroTxKey".to_string(),
                address: "moneroBridgeMultiSig".to_string(),
            },
            recipient: HumanAddr("mySecretAddress".to_string()),
            amount: Uint128(1000000),
            padding: None,
        };

        let result = handle(
            &mut deps,
            mock_env(bridge_minter.to_string(), &[]),
            mint_msg,
        );
        assert!(result.is_ok(), "Mint failed: {}", result.err().unwrap());

        let resp: HandleResult = from_binary(&result.unwrap().data.unwrap()).unwrap();
        match resp {
            HandleResult::MintSecretMonero { status } => {
                assert_eq!(status, ResponseStatus::Success)
            }
            _ => panic!("unexpected"),
        }
    }
}
