
use sc_finality_grandpa::GrandpaBlockImport;
use sp_api::Encode;
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
use sp_timestamp::InherentDataProvider as TimestampProvider;
use sp_runtime::traits::Header;
use sp_runtime::traits::UniqueSaturatedInto;
use std::sync::mpsc::{SyncSender, Receiver};

const N : u32 = 10;
const K : u32 = 4;

pub struct CasinoBlockImport<Backend, Block: BlockT, Client, SC> {
    grandpa_block_import : GrandpaBlockImport<Backend, Block, Client, SC>,
    block_timestamps : Vec<u128>,
    sender : SyncSender<CasinoMessage<Block>>,
}

impl<Backend, Block: BlockT, Client, SC: Clone> Clone
	for CasinoBlockImport<Backend, Block, Client, SC>
{
	fn clone(&self) -> Self {
		CasinoBlockImport {
		    grandpa_block_import : self.grandpa_block_import.clone(),
            block_timestamps : self.block_timestamps.clone(),
            sender : self.sender.clone(),
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
            let block_num : u32 = (*block.header.number()).unique_saturated_into();
            let timestamp : u128 = TimestampProvider::from_system_time().timestamp().as_duration().as_millis();

            self.block_timestamps.push(timestamp);

            if block_num == N {
                let mut avg_block_diff : u128 = 0;
                for i in 0..self.block_timestamps.len()-1 {
                    avg_block_diff += self.block_timestamps[i+1] - self.block_timestamps[i];
                }
                avg_block_diff /= (N - 1) as u128;
                let estimate = self.block_timestamps[self.block_timestamps.len() - 1] + K as u128 * avg_block_diff;

                info!("Casino Block Import estimate: {}", estimate);
                self.sender.send(CasinoMessage(estimate, block.header.hash())).unwrap();
            }

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
    sender : SyncSender<CasinoMessage<Block>>,
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

    Ok((CasinoBlockImport{grandpa_block_import, block_timestamps : Vec::<_>::new(), sender}, link_half))
}

use std::borrow::Cow;
use sc_network_gossip::{Validator, ValidatorContext, Network};
use prometheus_endpoint::Registry;
use sc_network::PeerId;
use futures_lite::future::FutureExt;

pub struct CasinoValidator;

impl<Block : BlockT> Validator<Block> for CasinoValidator {
    fn validate (
        &self,
        _context: &mut dyn ValidatorContext<Block>,
        who: &PeerId,
        data: &[u8],
        ) -> sc_network_gossip::ValidationResult<Block::Hash> {
            info!("Validating gossip message: ");
            info!("Author: {}", who);
            info!("Data: {:?}", data);
            sc_network_gossip::ValidationResult::Discard
    }
}

pub struct CasinoGossipEngine<B : BlockT> {
    gossip_engine : GossipEngine<B>,
    receiver : Receiver<CasinoMessage<B>>,
}

impl <B : BlockT> CasinoGossipEngine<B> {
    pub fn new<N: Network<B> + Send + Clone + 'static>(
        network: N,
        protocol: impl Into<Cow<'static, str>>,
        validator: Arc<dyn Validator<B>>,
        metrics_registry: Option<&Registry>,
        receiver : Receiver<CasinoMessage<B>>,
        ) -> Self {
        CasinoGossipEngine {
            gossip_engine : GossipEngine::new(network, protocol, validator, metrics_registry),
            receiver,
        }
    }
}

use std::task::Poll;
use std::task::Context;
use std::pin::Pin;
use std::future::Future;

pub struct CasinoMessage<B: BlockT>(u128, B::Hash);

impl<B: BlockT> Future for CasinoGossipEngine<B> {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.receiver.try_recv() {
                Ok(CasinoMessage(estimate, block_hash)) => {
                    info!("Casino Gossip Engine recieved estimate: {}", estimate);

                    self.gossip_engine.gossip_message(block_hash, estimate.encode(), false);

                    info!("Message gossiped");
                },
                _ => ()
            }

            match self.gossip_engine.poll(cx) {
                Poll::Pending => {
                    break;
                },
                _ => {
                    info!("Network closed");
                    return Poll::Ready(());
                }
            }
        }
        Poll::Pending
    }
}



