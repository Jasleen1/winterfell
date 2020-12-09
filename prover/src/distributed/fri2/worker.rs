use kompact::prelude::*;
use math::{field::{BaseElement, FieldElement}, quartic};
use crypto::{HashFunction, MerkleTree};
use common::fri_utils::hash_values;

// CONSTANTS
// ================================================================================================
const FOLDING_FACTOR: usize = 4;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Worker <E: FieldElement + From<BaseElement>> {
    ctx: ComponentContext<Self>,
    config: WorkerConfig,
    domain: Vec<BaseElement>,
    evaluations: Vec<Vec<[E; FOLDING_FACTOR]>>,
    remainder: E,
    trees: Vec<MerkleTree>,
}

pub struct WorkerConfig {
    pub domain_size: usize,
    pub num_partitions: usize,
    pub index: usize,
    pub hash_fn: HashFunction,
}

#[derive(Debug)]
pub enum ProverRequest {
    Commit,
    ApplyDrp(BaseElement),
    Query(Vec<usize>),
}

#[derive(Debug)]
pub enum WorkerResponse {
    Commit([u8; 32]),
    ApplyDrp,
    Query(Vec<Vec<QueryResult>>),
}

#[derive(Debug)]
pub struct QueryResult {
    pub index: usize,
    pub value: [BaseElement; 4],
    pub path: Vec<[u8; 32]>,
}

// WORKER IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>> Worker<E> {

    pub fn new(config: WorkerConfig) -> Self {
        Worker {
            ctx: ComponentContext::uninitialised(),
            config,
            domain: Vec::new(),
            evaluations: Vec::new(),
            remainder: E::ZERO,
            trees: vec![],
        }
    }

    pub fn commit(&mut self) -> [u8; 32] {
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

    pub fn apply_drp(&mut self, alpha: BaseElement) {
        let ys = &self.evaluations[self.evaluations.len() - 1];
        let xs = quartic::transpose(&self.domain, 1);

        let polys = quartic::interpolate_batch(&xs, ys);
        let evaluations = quartic::evaluate_batch(&polys, alpha);

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

    pub fn query(&self, positions: &[usize]) -> Vec<Vec<QueryResult>> {
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
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>> ComponentLifecycle for Worker<E> {}

impl<E: FieldElement + From<BaseElement>> Actor for Worker<E> {
    type Message = WithSender<ProverRequest, WorkerResponse>;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg.content() {
            ProverRequest::Commit => {
                let result = self.commit();
                msg.reply(WorkerResponse::Commit(result));
            }
            ProverRequest::ApplyDrp(alpha) => {
                self.apply_drp(*alpha);
                msg.reply(WorkerResponse::ApplyDrp);
            }
            ProverRequest::Query(positions) => {
                let result = self.query(positions);
                msg.reply(WorkerResponse::Query(result));
            }
        }
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}