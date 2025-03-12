use crate::alloy_db::{AlloyDBFork, WrapDatabaseAsync};
use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::map::{B256HashMap, HashMap};
use alloy_primitives::{Address, BlockNumber, Bytes, StorageKey, StorageValue, B256};
use alloy_provider::Provider;
use parking_lot::RwLock;
use reth_errors::{ProviderError, ProviderResult};
use reth_primitives::{Account, Bytecode};
use reth_provider::errors::any::AnyError;
use reth_provider::{
    AccountReader, BlockHashReader, HashedPostStateProvider, StateProofProvider, StateProvider, StateRootProvider, StorageRootProvider,
};
use reth_trie::updates::TrieUpdates;
use reth_trie::{AccountProof, HashedPostState, HashedStorage, MultiProof, MultiProofTargets, StorageMultiProof, StorageProof, TrieInput};
use revm::db::BundleState;
use revm::DatabaseRef;
use std::marker::PhantomData;
use tokio::runtime::Runtime;

pub struct AlloyRethStateProvider<N: Network, P: Provider<N> + Clone> {
    rt: Option<Runtime>,
    alloy_db: WrapDatabaseAsync<AlloyDBFork<N, P>>,
    bytecode: RwLock<HashMap<B256, Bytecode>>,
    _n: PhantomData<N>,
}

impl<N: Network, P: Provider<N> + Clone> AlloyRethStateProvider<N, P> {
    pub fn new(provider: P, block_id: BlockId) -> Self {
        let rt = Runtime::new().unwrap();
        let alloy_db = AlloyDBFork::new(provider.clone(), block_id);
        let wrapped_db = WrapDatabaseAsync::with_handle(alloy_db, rt.handle().clone());
        Self { rt: Some(rt), alloy_db: wrapped_db, bytecode: RwLock::new(HashMap::default()), _n: PhantomData }
    }
}

impl<N: Network, P: Provider<N> + Clone> Drop for AlloyRethStateProvider<N, P> {
    fn drop(&mut self) {
        if let Some(runtime) = self.rt.take() {
            runtime.shutdown_background();
        }
    }
}

impl<N, P> BlockHashReader for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn block_hash(&self, number: BlockNumber) -> ProviderResult<Option<B256>> {
        match self.alloy_db.block_hash_ref(number) {
            Ok(value) => Ok(Some(value)),
            Err(e) => Err(ProviderError::Other(AnyError::new(e))),
        }
    }

    fn canonical_hashes_range(&self, _start: BlockNumber, _end: BlockNumber) -> ProviderResult<Vec<B256>> {
        todo!()
    }
}

impl<N, P> AccountReader for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn basic_account(&self, address: &Address) -> ProviderResult<Option<Account>> {
        match self.alloy_db.basic_ref(*address) {
            Ok(Some(account)) => {
                let bytecode_hash = match account.code {
                    Some(code) => {
                        if account.code_hash == KECCAK_EMPTY {
                            None
                        } else {
                            self.bytecode.write().insert(account.code_hash, Bytecode::new_raw(code.bytes()));
                            Some(account.code_hash)
                        }
                    }
                    None => None,
                };

                Ok(Some(Account { nonce: account.nonce, balance: account.balance, bytecode_hash }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ProviderError::Other(AnyError::new(e))),
        }
    }
}

impl<N, P> StateRootProvider for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn state_root(&self, _hashed_state: HashedPostState) -> ProviderResult<B256> {
        todo!()
    }

    fn state_root_from_nodes(&self, _input: TrieInput) -> ProviderResult<B256> {
        todo!()
    }

    fn state_root_with_updates(&self, _hashed_state: HashedPostState) -> ProviderResult<(B256, TrieUpdates)> {
        todo!()
    }

    fn state_root_from_nodes_with_updates(&self, _input: TrieInput) -> ProviderResult<(B256, TrieUpdates)> {
        todo!()
    }
}

impl<N, P> StorageRootProvider for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn storage_root(&self, _address: Address, _hashed_storage: HashedStorage) -> ProviderResult<B256> {
        todo!()
    }

    fn storage_proof(&self, _address: Address, _slot: B256, _hashed_storage: HashedStorage) -> ProviderResult<StorageProof> {
        todo!()
    }

    fn storage_multiproof(&self, _address: Address, _slots: &[B256], _hashed_storage: HashedStorage) -> ProviderResult<StorageMultiProof> {
        todo!()
    }
}

impl<N, P> StateProofProvider for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn proof(&self, _input: TrieInput, _address: Address, _slots: &[B256]) -> ProviderResult<AccountProof> {
        todo!()
    }

    fn multiproof(&self, _input: TrieInput, _targets: MultiProofTargets) -> ProviderResult<MultiProof> {
        todo!()
    }

    fn witness(&self, _input: TrieInput, _target: HashedPostState) -> ProviderResult<B256HashMap<Bytes>> {
        todo!()
    }
}

impl<N, P> HashedPostStateProvider for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Clone + Provider<N>,
{
    fn hashed_post_state(&self, _bundle_state: &BundleState) -> HashedPostState {
        todo!()
    }
}

impl<N, P> StateProvider for AlloyRethStateProvider<N, P>
where
    N: Network,
    P: Provider<N> + Clone,
{
    fn storage(&self, account: Address, storage_key: StorageKey) -> ProviderResult<Option<StorageValue>> {
        match self.alloy_db.storage_ref(account, storage_key.into()) {
            Ok(value) => Ok(Some(value)),
            Err(e) => Err(ProviderError::Other(AnyError::new(e))),
        }
    }

    // Will be easier with https://github.com/paradigmxyz/reth/issues/14479
    fn bytecode_by_hash(&self, code_hash: &B256) -> ProviderResult<Option<Bytecode>> {
        // revm will first call account info, which will insert the bytecode into the hashmap
        Ok(self.bytecode.read().get(code_hash).cloned())
    }
}
