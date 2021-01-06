use super::messages::{ManagerMessage, QueryResult, WorkerMessage};
use crate::{folding::quartic, utils::hash_values};
use crypto::{HashFunction, MerkleTree};
use fasthash::xx::Hash64;
use kompact::prelude::*;
use log::debug;
use math::field::{BaseElement, FieldElement, StarkField};
use std::{collections::HashSet, sync::Arc};

// CONSTANTS
// ================================================================================================
const FOLDING_FACTOR: usize = 4;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Worker {
    ctx: ComponentContext<Self>,
    config: WorkerConfig,
    domain: Vec<BaseElement>,
    evaluations: Vec<Vec<[BaseElement; FOLDING_FACTOR]>>,
    remainder: BaseElement,
    trees: Vec<MerkleTree>,
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
            domain: Vec::new(),
            evaluations: Vec::new(),
            remainder: BaseElement::ZERO,
            trees: vec![],
        }
    }

    /// Prepares the worker for a new invocation of FRI protocol.
    fn prepare(&mut self, evaluations: Arc<Vec<BaseElement>>) {
        // build a domain for the top layer of evaluations
        let global_domain_size = evaluations.len();
        let g = BaseElement::get_root_of_unity(global_domain_size.trailing_zeros());
        let stride = g.exp(self.config.num_partitions as u128);
        let mut x = g.exp(self.config.index as u128);

        let domain_size = global_domain_size / self.config.num_partitions;
        self.domain = Vec::with_capacity(domain_size);
        self.domain.push(x);
        for _ in 1..domain_size {
            x = x * stride;
            self.domain.push(x);
        }

        // make a copy of evaluations relevant for this worker
        let mut partition = Vec::new();
        for i in (self.config.index..evaluations.len()).step_by(self.config.num_partitions) {
            partition.push(evaluations[i]);
        }
        self.evaluations = vec![quartic::transpose(&partition, 1)];

        // reset all other properties
        self.trees.clear();
        self.remainder = BaseElement::ZERO;
    }

    /// Commit to the current set of evaluations by putting them into a Merkle tree
    /// and returning the root of this tree.
    fn commit(&mut self) -> [u8; 32] {
        let evaluations = &self.evaluations[self.evaluations.len() - 1];
        let hashed_evaluations = hash_values(&evaluations, self.config.hash_fn);
        if hashed_evaluations.len() == 1 {
            hashed_evaluations[0]
        } else {
            let evaluation_tree = MerkleTree::new(hashed_evaluations, self.config.hash_fn);
            let root = *evaluation_tree.root();
            self.trees.push(evaluation_tree);
            root
        }
    }

    fn apply_drp(&mut self, alpha: BaseElement) {
        let ys = &self.evaluations[self.evaluations.len() - 1];
        let xs = quartic::transpose(&self.domain, 1);

        let polys = quartic::interpolate_batch(&xs, ys);
        let evaluations = quartic::evaluate_batch(&polys, alpha.into());

        if evaluations.len() == 1 {
            self.remainder = evaluations[0];
        } else {
            self.evaluations.push(quartic::transpose(&evaluations, 1));
        }

        self.domain = self
            .domain
            .iter()
            .take(self.domain.len() / FOLDING_FACTOR)
            .map(|&x| x.exp(FOLDING_FACTOR as u128))
            .collect();
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
    type Message = WithSenderStrong<WorkerMessage, ManagerMessage>;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg.content() {
            WorkerMessage::Prepare(evaluations) => {
                debug!("worker {}: Prepare message received", self.config.index);
                self.prepare(evaluations.clone());
                msg.reply(ManagerMessage::WorkerReady(self.config.index));
            }
            WorkerMessage::CommitToLayer => {
                debug!(
                    "worker {}: CommitToLayer message received",
                    self.config.index
                );
                let result = self.commit();
                msg.reply(ManagerMessage::WorkerCommit(self.config.index, result));
            }
            WorkerMessage::ApplyDrp(alpha) => {
                debug!("worker {}: ApplyDrp message received", self.config.index);
                self.apply_drp(*alpha);
                msg.reply(ManagerMessage::WorkerDrpComplete(self.config.index));
            }
            WorkerMessage::RetrieveRemainder => {
                debug!(
                    "worker {}: RetrieveRemainder message received",
                    self.config.index
                );
                msg.reply(ManagerMessage::WorkerRemainder(
                    self.config.index,
                    self.remainder,
                ));
            }
            WorkerMessage::Query(positions) => {
                debug!("worker {}: Query message received", self.config.index);
                let result = self.query(positions);
                msg.reply(ManagerMessage::WorkerQueryResult(self.config.index, result));
            }
        }
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}
