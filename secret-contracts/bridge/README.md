# Secret Monero Bridge Contract

## Instantiate snip20
1. Get code_id of already deployed snip20 or re-deploy
1. Instantiate:
```
secretcli tx compute instantiate ${code_id} \
'{
  "name": "secretMonero",
  "symbol": "SXMR",
  "decimals": 12,
  "prng_seed": ${base64_encoded},
  "config": {
    "public_total_supply": true,
    "enable_deposit": false,
    "enable_redeem": false,
    "enable_mint": true,
    "enable_burn": true
  }
}' \
 --label ${your_label} --from ${your_alias_or_address} --gas 180000 -y
```
1. Fetch contract_address: `secretcli q compute list-contract-by-code ${code_id}`
1. Fetch contract_hash: `secretcli q compute contract-hash ${contract_address}`

## Deploy && Instantiate bridge contract

1. Deploy: `secretcli tx compute store contract.wasm.gz --from ${your_account_alias} -y --gas 1000000 --gas-prices=1.0uscrt`
1. Fetch code_id: `secretcli query tx ${tx_hash}`
1. Init:
```
secretcli tx compute instantiate ${code_id} \
'{
  "min_swap_amount": ${min_swappable_sxmr_amount},
  "viewing_key": "${your_vk}", // TODO remove
  "secret_monero": {
    "address": "${sxmr_snip20_address}",
    "contract_hash": "${sxmr_snip20_hash}"
  },
  "prng_seed": "${base64_encoded}"
}'\
 --label ${your_label} --from ${your_alias} --gas 180000 -y
```

## Set bridge contract as a minter
1. `secretcli tx compute execute ${sXMR_address} '{"set_minters":{"minters":["${bridge_contract_address}"]}}' --from ${your_alias} --gas 130000 -y`

## Add bridge multi-sig wallet as a minter (this must be owned by the nodes that verify the monero_proofs)
1. `secretcli tx compute execute ${bridge_contract_address} '{"set_minters":{"minters":["${multisig_address}"]}}' --from ${your_alias} --gas 130000 -y`
