
use sc_finality_grandpa::GrandpaBlockImport;
use sp_blockchain::Error as ClientError;
use sp_blockchain::well_known_cache_keys;
use sc_consensus::block_import::{JustificationImport, BlockImport};
use sc_consensus::block_import::{ImportResult, BlockImportParams, BlockCheckParams};
use sp_api::NumberFor;
use sc_client_api::Backend;
use sc_finality_grandpa::ClientForGrandpa;
use sp_consensus::SelectChain;
use sp_consensus::Error as ConsensusError;
use sc_finality_grandpa::GrandpaApi;
use sp_api::TransactionFor;
use std::collections::HashMap;
use std::sync::Arc;
use sc_finality_grandpa::GenesisAuthoritySetProvider;
use sc_finality_grandpa::LinkHalf;
use sc_telemetry::TelemetryHandle;
use sp_api::BlockT;
use sp_runtime::Justification;
use log::info;
use sc_network_gossip::GossipEngine;

const N : u32 = 10;
const K : u32 = 4;

pub struct CasinoBlockImport<Backend, Block: BlockT, Client, SC> {
    grandpa_block_import : GrandpaBlockImport<Backend, Block, Client, SC>,
}

impl<Backend, Block: BlockT, Client, SC: Clone> Clone
	for CasinoBlockImport<Backend, Block, Client, SC>
{
	fn clone(&self) -> Self {
		CasinoBlockImport {
		    grandpa_block_import : self.grandpa_block_import.clone()
		}
	}
}



#[async_trait::async_trait]
impl<BE, Block: BlockT, Client, SC> JustificationImport<Block>
	for CasinoBlockImport<BE, Block, Client, SC>
where
	NumberFor<Block>: sc_finality_grandpa::BlockNumberOps,
	BE: Backend<Block> + sc_client_api::backend::Backend<Block>,
	Client: ClientForGrandpa<Block, BE>,
	SC: SelectChain<Block>,
{
	type Error = ConsensusError;

	async fn on_start(&mut self) -> Vec<(Block::Hash, NumberFor<Block>)> {
        self.grandpa_block_import.on_start().await
	}

	async fn import_justification(
		&mut self,
		hash: Block::Hash,
		number: NumberFor<Block>,
		justification: Justification,
	) -> Result<(), Self::Error> {
		self.grandpa_block_import.import_justification(hash, number, justification).await
	}
}


#[async_trait::async_trait]
impl<BE, Block: BlockT, Client, SC> BlockImport<Block> for CasinoBlockImport<BE, Block, Client, SC>
where
	NumberFor<Block>: sc_finality_grandpa::BlockNumberOps,
	BE: Backend<Block> + sc_client_api::backend::Backend<Block>,
	Client: ClientForGrandpa<Block, BE>,
	Client::Api: GrandpaApi<Block>,
	for<'a> &'a Client:
		BlockImport<Block, Error = ConsensusError, Transaction = TransactionFor<Client, Block>>,
	TransactionFor<Client, Block>: 'static,
	SC: Send,
{
	type Error = ConsensusError;
	type Transaction = TransactionFor<Client, Block>;

	async fn import_block(
		&mut self,
		block: BlockImportParams<Block, Self::Transaction>,
		new_cache: HashMap<well_known_cache_keys::Id, Vec<u8>>,
	) -> Result<ImportResult, Self::Error> {
            info!("Casino Block Import");

	        self.grandpa_block_import.import_block(block, new_cache).await
	}

	async fn check_block(
		&mut self,
		block: BlockCheckParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		self.grandpa_block_import.check_block(block).await
	}
}

pub fn casino_block_import<BE, Block: BlockT, Client, SC>(
	client: Arc<Client>,
	genesis_authorities_provider: &dyn GenesisAuthoritySetProvider<Block>,
	select_chain: SC,
	telemetry: Option<TelemetryHandle>,
) -> Result<(CasinoBlockImport<BE, Block, Client, SC>, LinkHalf<Block, Client, SC>), ClientError>
where
	SC: SelectChain<Block>,
	BE: Backend<Block> + 'static + sc_client_api::backend::Backend<Block>,
	Client: ClientForGrandpa<Block, BE> + 'static,
{
	let (grandpa_block_import, link_half) = sc_finality_grandpa::block_import(
		client,
		genesis_authorities_provider,
		select_chain,
		telemetry,
	)?;

    Ok((CasinoBlockImport{grandpa_block_import}, link_half))
}


struct CasinoGossipEngine<B : BlockT> {
    gossip_engine : GossipEngine<B>,
}


