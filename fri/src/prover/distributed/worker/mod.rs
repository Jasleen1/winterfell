use super::{
    messages::{WorkerCheckIn, WorkerPartitions, WorkerRequest, WorkerResponse}
};
use crypto::HashFunction;
use kompact::prelude::*;
use std::sync::Arc;

mod partition;
use partition::Partition;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Worker {
    ctx: ComponentContext<Self>,
    prover: ActorPath,
    hash_fn: HashFunction,
    partitions: Vec<Partition>,
}

// WORKER IMPLEMENTATION
// ================================================================================================

impl Worker {
    pub fn new(prover: ActorPath, hash_fn: HashFunction) -> Self {
        Worker {
            ctx: ComponentContext::uninitialised(),
            prover,
            hash_fn,
            partitions: Vec::new(),
        }
    }

    /// Prepares the worker for a new invocation of FRI protocol.
    fn prepare(&mut self, worker_partitions: WorkerPartitions) {
        self.partitions.clear();
        let evaluations = Arc::new(worker_partitions.evaluations);
        for &partition_idx in worker_partitions.partition_indexes.iter() {
            self.partitions.push(Partition::new(
                partition_idx,
                worker_partitions.num_partitions,
                evaluations.clone(),
            ));
        }
        self.prover.tell(WorkerResponse::WorkerReady, self);
    }
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl ComponentLifecycle for Worker {
    fn on_start(&mut self) -> Handled {
        self.prover.tell(WorkerCheckIn, self);
        Handled::Ok
    }
}

impl NetworkActor for Worker {
    type Message = WorkerRequest;
    type Deserialiser = WorkerRequest;

    fn receive(&mut self, sender: Option<ActorPath>, request: Self::Message) -> Handled {
        if let Some(prover) = sender {
            match request {
                WorkerRequest::Prepare(worker_partitions) => {
                    debug!(
                        self.ctx.log(),
                        "Prepare message received with partitions {:?}",
                        worker_partitions.partition_indexes
                    );
                    self.prepare(worker_partitions);
                }
                WorkerRequest::CommitToLayer => {
                    debug!(self.ctx.log(), "CommitToLayer message received");
                    let hash_fn = self.hash_fn;
                    let commitments = self
                        .partitions
                        .iter_mut()
                        .map(|p| p.commit_layer(hash_fn))
                        .collect();
                    prover.tell(WorkerResponse::CommitResult(commitments), self);
                }
                WorkerRequest::ApplyDrp(alpha) => {
                    debug!(
                        self.ctx.log(),
                        "ApplyDrp message received with alpha: {}", alpha
                    );
                    for partition in self.partitions.iter_mut() {
                        partition.apply_drp(alpha);
                    }
                    prover.tell(WorkerResponse::DrpComplete, self);
                }
                WorkerRequest::RetrieveRemainder => {
                    debug!(self.ctx.log(), "RetrieveRemainder message received");
                    let remainder = self.partitions.iter().map(|p| p.remainder()).collect();
                    prover.tell(WorkerResponse::RemainderResult(remainder), self);
                }
                WorkerRequest::Query(positions) => {
                    debug!(self.ctx.log(), "Query message received");
                    let results = self
                        .partitions
                        .iter()
                        .map(|p| p.query(&positions))
                        .collect();
                    prover.tell(WorkerResponse::QueryResult(results), self);
                }
            }
        }

        Handled::Ok
    }
}
