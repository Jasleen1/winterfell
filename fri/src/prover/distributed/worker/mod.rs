use super::{
    messages::{WorkerCheckIn, WorkerPartitions, WorkerRequest, WorkerResponse},
    ProtocolStep,
};
use crypto::HashFunction;
use kompact::prelude::*;
use math::field::BaseElement;
use std::sync::Arc;

mod partition;
use partition::Partition;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Worker {
    ctx: ComponentContext<Self>,
    state: ProtocolStep,
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
            state: ProtocolStep::None,
            prover,
            hash_fn,
            partitions: Vec::new(),
        }
    }

    /// Prepares the worker for a new invocation of FRI protocol.
    fn handle_assign_partitions(&mut self, partitions: WorkerPartitions) {
        debug!(
            self.ctx.log(),
            "AssignPartitions message received with partitions {:?}", partitions.partition_indexes
        );

        // make sure we are not in the middle of executing another request
        let next_state = ProtocolStep::Ready(partitions.num_layers);
        assert!(
            self.state == ProtocolStep::None,
            "invalid state transition from {:?} to {:?}",
            self.state,
            next_state,
        );
        self.state = next_state;

        // build a set of partitions for this worker and notify the prover once
        // all partition data has been localized
        let evaluations = Arc::new(partitions.evaluations);
        for &partition_idx in partitions.partition_indexes.iter() {
            self.partitions.push(Partition::new(
                partition_idx,
                partitions.num_partitions,
                evaluations.clone(),
            ));
        }
        self.prover.tell(WorkerResponse::WorkerReady, self);
    }

    /// Commits to the current layer for all partitions assigned to the worker and sends
    /// the results (Merkle tree roots) to the prover actor.
    fn handle_commit_to_layer(&mut self, prover: ActorPath) {
        debug!(self.ctx.log(), "CommitToLayer message received");

        // make sure commit_to_layer can be invoked at this point in the protocol
        let next_state = self.state.get_next_state();
        if let ProtocolStep::LayerCommitted(_) = next_state {
            self.state = next_state;
        } else {
            panic!(
                "invalid state transition from {:?} to {:?}",
                self.state, next_state
            );
        }

        // compute commitments for all partitions
        let hash_fn = self.hash_fn;
        let commitments = self
            .partitions
            .iter_mut()
            .map(|p| p.commit_layer(hash_fn))
            .collect();

        // send the result to the prover
        prover.tell(WorkerResponse::CommitResult(commitments), self);
    }

    /// Applies degree-respecting projection for all partitions assigned to the worker.
    fn handle_apply_drp(&mut self, alpha: BaseElement, prover: ActorPath) {
        debug!(
            self.ctx.log(),
            "ApplyDrp message received with alpha: {}", alpha
        );

        // make sure apply_drp can be invoked at this point in the protocol
        let next_state = self.state.get_next_state();
        if let ProtocolStep::DrpApplied(_) = next_state {
            self.state = next_state;
        } else {
            panic!(
                "invalid state transition from {:?} to {:?}",
                self.state, next_state
            );
        }

        // apply degree-respecting project for all partitions
        for partition in self.partitions.iter_mut() {
            partition.apply_drp(alpha);
        }

        // notify the prover when the work is done
        prover.tell(WorkerResponse::DrpComplete, self);
    }

    /// Sends the value of the remainder to the prover.
    fn handle_retrieve_remainder(&mut self, prover: ActorPath) {
        debug!(self.ctx.log(), "RetrieveRemainder message received");
        assert!(
            self.state.is_completed(),
            "cannot retrieve remainder; layer reduction hasn't been completed yet"
        );

        // collect remainders from all partitions and send them to the prover
        let remainder = self.partitions.iter().map(|p| p.remainder()).collect();
        prover.tell(WorkerResponse::RemainderResult(remainder), self);
    }

    /// Queries all layers for all partitions assigned to the work and sends query
    /// results to the prover.
    fn handle_query(&mut self, positions: Vec<usize>, prover: ActorPath) {
        debug!(self.ctx.log(), "Query message received");
        assert!(
            self.state.is_completed(),
            "cannot query layers; layer reduction hasn't been completed yet"
        );

        // collect query results from all partitions and send them to the prover
        let results = self
            .partitions
            .iter()
            .map(|p| p.query(&positions))
            .collect();
        prover.tell(WorkerResponse::QueryResult(results), self);
    }

    /// Clears the worker's state
    fn handle_reset(&mut self) {
        self.state = ProtocolStep::None;
        self.partitions.clear();
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
                WorkerRequest::AssignPartitions(partitions) => {
                    self.handle_assign_partitions(partitions)
                }
                WorkerRequest::CommitToLayer => self.handle_commit_to_layer(prover),
                WorkerRequest::ApplyDrp(alpha) => self.handle_apply_drp(alpha, prover),
                WorkerRequest::RetrieveRemainder => self.handle_retrieve_remainder(prover),
                WorkerRequest::Query(positions) => self.handle_query(positions, prover),
                WorkerRequest::Reset => self.handle_reset(),
            }
        } else {
            panic!("no sender provided for worker message");
        }

        Handled::Ok
    }
}
