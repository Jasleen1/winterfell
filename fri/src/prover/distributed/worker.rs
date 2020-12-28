use crypto::{HashFunction, MerkleTree};
use fasthash::xx::Hash64;
use super::{
    messages::{ManagerMessage, QueryResult, WorkerPartitions, WorkerRequest, WorkerResponse},
    partition::Partition,
};
use crypto::HashFunction;
use kompact::prelude::*;
use log::debug;
use math::field::BaseElement;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Worker {
    ctx: ComponentContext<Self>,
    index: usize,
    hash_fn: HashFunction,
    partitions: Vec<Partition>,
}

// WORKER IMPLEMENTATION
// ================================================================================================

impl Worker {
    pub fn new(index: usize, hash_fn: HashFunction) -> Self {
        Worker {
            ctx: ComponentContext::uninitialised(),
            index,
            hash_fn,
            partitions: Vec::new(),
        }
    }

    /// Prepares the worker for a new invocation of FRI protocol.
    fn prepare(&mut self, worker_partitions: &WorkerPartitions) {
        self.partitions.clear();
        for &partition_idx in worker_partitions.partition_indexes.iter() {
            self.partitions.push(Partition::new(
                partition_idx,
                worker_partitions.num_partitions,
                worker_partitions.evaluations.clone(),
            ));
        }
    }

    /// Commit to the current set of evaluations by putting them into a Merkle tree
    /// and returning the root of this tree.
    fn commit(&mut self) -> Vec<[u8; 32]> {
        let hash_fn = self.hash_fn;
        self.partitions
            .iter_mut()
            .map(|p| p.commit_layer(hash_fn))
            .collect::<Vec<_>>()
    }

    fn apply_drp(&mut self, alpha: BaseElement) {
        for partition in self.partitions.iter_mut() {
            partition.apply_drp(alpha);
        }
    }

    fn query(&self, positions: &[usize]) -> Vec<Vec<Vec<QueryResult>>> {
        self.partitions
            .iter()
            .map(|p| p.query(positions))
            .collect::<Vec<_>>()
    }
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl ComponentLifecycle for Worker {}

impl Actor for Worker {
    type Message = WithSenderStrong<WorkerRequest, ManagerMessage>;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg.content() {
            WorkerRequest::Prepare(worker_partitions) => {
                debug!(
                    "worker {}: Prepare message received with partitions {:?}",
                    self.index, worker_partitions.partition_indexes
                );
                self.prepare(&worker_partitions);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::WorkerReady(
                    self.index,
                )));
            }
            WorkerRequest::CommitToLayer => {
                debug!("worker {}: CommitToLayer message received", self.index);
                let result = self.commit();
                msg.reply(ManagerMessage::WorkerResponse(
                    WorkerResponse::CommitResult(self.index, result),
                ));
            }
            WorkerRequest::ApplyDrp(alpha) => {
                debug!("worker {}: ApplyDrp message received", self.index);
                self.apply_drp(*alpha);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::DrpComplete(
                    self.index,
                )));
            }
            WorkerRequest::RetrieveRemainder => {
                debug!("worker {}: RetrieveRemainder message received", self.index);
                let remainder = self.partitions.iter().map(|p| p.remainder()).collect();
                msg.reply(ManagerMessage::WorkerResponse(
                    WorkerResponse::RemainderResult(self.index, remainder),
                ));
            }
            WorkerRequest::Query(positions) => {
                debug!("worker {}: Query message received", self.index);
                let result = self.query(positions);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::QueryResult(
                    self.index, result,
                )));
            }
        }
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}
