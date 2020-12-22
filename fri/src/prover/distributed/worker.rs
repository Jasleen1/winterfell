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
    config: WorkerConfig,
    partitions: Vec<Partition>,
}

pub struct WorkerConfig {
    pub num_partitions: usize,
    pub index: usize,
    pub hash_fn: HashFunction,
}

// WORKER IMPLEMENTATION
// ================================================================================================

impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        Worker {
            ctx: ComponentContext::uninitialised(),
            config,
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
    fn commit(&mut self) -> [u8; 32] {
        let hash_fn = self.config.hash_fn;
        let result = self
            .partitions
            .iter_mut()
            .map(|p| p.commit_layer(hash_fn))
            .collect::<Vec<_>>();
        result[0] // TODO: return the full array
    }

    fn apply_drp(&mut self, alpha: BaseElement) {
        for partition in self.partitions.iter_mut() {
            partition.apply_drp(alpha);
        }
    }

    fn query(&self, positions: &[usize]) -> Vec<Vec<QueryResult>> {
        // filter out positions which don't belong to this worker, and if there is
        // nothing to query, return with empty vector
        let mut positions = self.to_local_positions(positions);
        if positions.is_empty() {
            return vec![];
        }

        let mut result = Vec::new();
        for (layer_depth, evaluations) in self.evaluations.iter().enumerate() {
            positions = self.map_positions(&positions, layer_depth);
            let mut layer_result = Vec::new();
            for &position in positions.iter() {
                let path = if layer_depth < self.trees.len() {
                    self.trees[layer_depth].prove(position)
                } else {
                    Vec::new()
                };

                layer_result.push(QueryResult {
                    value: evaluations[position],
                    path,
                    index: position,
                });
            }
            result.push(layer_result);
        }

        result
    }

    fn to_local_positions(&self, positions: &[usize]) -> Vec<usize> {
        let mut local_positions = HashSet::with_hasher(Hash64);
        for &p in positions.iter() {
            if p % self.config.num_partitions == self.config.index {
                local_positions.insert((p - self.config.index) / self.config.num_partitions);
            }
        }
        local_positions.into_iter().collect()
    }

    fn map_positions(&self, positions: &[usize], layer_depth: usize) -> Vec<usize> {
        let mut result = HashSet::with_hasher(Hash64);
        let num_evaluations = self.evaluations[layer_depth].len();
        positions.iter().for_each(|p| {
            result.insert(p % num_evaluations);
        });
        result.into_iter().collect::<Vec<_>>()
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
                    self.config.index, worker_partitions.partition_indexes
                );
                self.prepare(&worker_partitions);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::WorkerReady(
                    self.config.index,
                )));
            }
            WorkerRequest::CommitToLayer => {
                debug!(
                    "worker {}: CommitToLayer message received",
                    self.config.index
                );
                let result = self.commit();
                msg.reply(ManagerMessage::WorkerResponse(
                    WorkerResponse::CommitResult(self.config.index, result),
                ));
            }
            WorkerRequest::ApplyDrp(alpha) => {
                debug!("worker {}: ApplyDrp message received", self.config.index);
                self.apply_drp(*alpha);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::DrpComplete(
                    self.config.index,
                )));
            }
            WorkerRequest::RetrieveRemainder => {
                debug!(
                    "worker {}: RetrieveRemainder message received",
                    self.config.index
                );
                let p = &self.partitions[0];
                msg.reply(ManagerMessage::WorkerResponse(
                    WorkerResponse::RemainderResult(p.index(), p.remainder()),
                ));
            }
            WorkerRequest::Query(positions) => {
                debug!("worker {}: Query message received", self.config.index);
                let result = self.query(positions);
                msg.reply(ManagerMessage::WorkerResponse(WorkerResponse::QueryResult(
                    self.config.index,
                    result,
                )));
            }
        }
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}
