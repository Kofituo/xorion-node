use crate::{AccountId, BalancesConfig, RuntimeGenesisConfig, SudoConfig};
use alloc::{vec, vec::Vec};
use frame_support::build_struct_json_patch;
use serde_json::Value;
use sp_consensus_babe::{AuthorityId as BabeId, BabeAuthorityWeight};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_genesis_builder::{self, PresetId};
use sp_keyring::Sr25519Keyring;

// Returns the genesis config presets populated with given parameters.
fn testnet_genesis(
    initial_authorities: Vec<(BabeId, GrandpaId)>,
    endowed_accounts: Vec<AccountId>,
    root: AccountId,
) -> Value {
    build_struct_json_patch!(RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1u128 << 60))
                .collect::<Vec<_>>(),
        },
        babe: pallet_babe::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .enumerate()
                .map(|(x, (babe, _))| (babe.clone(), x as BabeAuthorityWeight + 1))
                .collect::<Vec<_>>(),
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect::<Vec<_>>(),
        },
        sudo: SudoConfig { key: Some(root) },
    })
}

/// Return the development genesis config.
pub fn development_config_genesis() -> Value {
    testnet_genesis(
        vec![(
            sp_keyring::Sr25519Keyring::Alice.public().into(),
            sp_keyring::Ed25519Keyring::Alice.public().into(),
        )],
        vec![
            Sr25519Keyring::Alice.to_account_id(),
            Sr25519Keyring::Bob.to_account_id(),
            Sr25519Keyring::AliceStash.to_account_id(),
            Sr25519Keyring::BobStash.to_account_id(),
        ],
        sp_keyring::Sr25519Keyring::Alice.to_account_id(),
    )
}

/// Return the local genesis config preset.
pub fn local_config_genesis() -> Value {
    testnet_genesis(
        vec![
            (
                sp_keyring::Sr25519Keyring::Alice.public().into(),
                sp_keyring::Ed25519Keyring::Alice.public().into(),
            ),
            (
                sp_keyring::Sr25519Keyring::Bob.public().into(),
                sp_keyring::Ed25519Keyring::Bob.public().into(),
            ),
        ],
        Sr25519Keyring::iter()
            .filter(|v| v != &Sr25519Keyring::One && v != &Sr25519Keyring::Two)
            .map(|v| v.to_account_id())
            .collect::<Vec<_>>(),
        Sr25519Keyring::Alice.to_account_id(),
    )
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
    let patch = match id.as_ref() {
        sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
        sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_config_genesis(),
        _ => return None,
    };
    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
    vec![
        PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
    ]
}
