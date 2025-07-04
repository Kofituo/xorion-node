//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use jsonrpsee::RpcModule;
use sc_client_api::AuxStore;
use sc_consensus_babe::BabeWorkerHandle;
use sc_consensus_grandpa::{
    FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_keystore::KeystorePtr;
use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};
use xorion_runtime::{opaque::Block, AccountId, Balance, BlockNumber, Hash, Nonce};

/// Extra dependencies for BABE.
pub struct BabeDeps {
    /// A handle to the BABE worker for issuing requests.
    pub babe_worker_handle: BabeWorkerHandle<Block>,
    /// The keystore that manages the keys of the node.
    pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
    /// Voting round info.
    pub shared_voter_state: SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: GrandpaJustificationStream<Block>,
    /// Executor to drive the subscription manager in the Grandpa RPC handler.
    pub subscription_executor: SubscriptionTaskExecutor,
    /// Finality proof provider.
    pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// The SelectChain Strategy
    pub select_chain: SC,
    /// A copy of the chain spec.
    pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
    /// BABE specific dependencies.
    pub babe: BabeDeps,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps<B>,
    /// The backend used by the node.
    pub backend: Arc<B>,
}
/// Instantiate all full RPC extensions.
pub fn create_full<C, P, SC, B>(
    FullDeps { client, pool, select_chain, chain_spec, babe, grandpa, backend }: FullDeps<
        C,
        P,
        SC,
        B,
    >,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + sc_client_api::BlockBackend<Block>
        + HeaderBackend<Block>
        + AuxStore
        + HeaderMetadata<Block, Error = BlockChainError>
        + Sync
        + Send
        + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: mmr_rpc::MmrRuntimeApi<Block, <Block as sp_runtime::traits::Block>::Hash, BlockNumber>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BabeApi<Block>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
    SC: SelectChain<Block> + 'static,
    B: sc_client_api::Backend<Block> + Send + Sync + 'static,
    B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>, {
    use mmr_rpc::{Mmr, MmrApiServer};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use sc_consensus_babe_rpc::{Babe, BabeApiServer};
    use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
    use sc_rpc::dev::{Dev, DevApiServer};
    use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcModule::new(());

    let BabeDeps { keystore, babe_worker_handle } = babe;
    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscription_executor,
        finality_provider,
    } = grandpa;

    io.merge(System::new(client.clone(), pool).into_rpc())?;
    // Making synchronous calls in light client freezes the browser currently,
    // more context: https://github.com/paritytech/substrate/pull/3480
    // These RPCs should use an asynchronous caller instead.
    io.merge(
        Mmr::new(
            client.clone(),
            backend
                .offchain_storage()
                .ok_or("Backend doesn't provide an offchain storage")?,
        )
        .into_rpc(),
    )?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    io.merge(
        Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain).into_rpc(),
    )?;
    io.merge(
        Grandpa::new(
            subscription_executor,
            shared_authority_set.clone(),
            shared_voter_state,
            justification_stream,
            finality_provider,
        )
        .into_rpc(),
    )?;

    io.merge(
        SyncState::new(chain_spec, client.clone(), shared_authority_set, babe_worker_handle)?
            .into_rpc(),
    )?;

    io.merge(StateMigration::new(client.clone(), backend).into_rpc())?;
    io.merge(Dev::new(client).into_rpc())?;
    Ok(io)
}
