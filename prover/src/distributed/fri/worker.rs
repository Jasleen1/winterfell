use super::FOLDING_FACTOR;
use crypto::{HashFunction, MerkleTree};
use fri::utils::hash_values;
use math::{
    field::{BaseElement, FieldElement, StarkField},
    quartic,
};
use std::collections::HashSet;

// TYPES AND INTERFACES
// ================================================================================================

pub struct Worker<E: FieldElement + From<BaseElement>> {
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
pub struct QueryResult<E: FieldElement> {
    pub index: usize,
    pub value: [E; FOLDING_FACTOR],
    pub path: Vec<[u8; 32]>,
}

// WORKER STRUCT AND IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>> Worker<E> {
    pub fn new(config: WorkerConfig, evaluations: &[E]) -> Self {
        let g = BaseElement::get_root_of_unity(config.domain_size.trailing_zeros());
        let stride = g.exp(config.num_partitions as u128);
        let mut x = g.exp(config.index as u128);

        let mut domain = vec![x];
        for _ in 1..evaluations.len() {
            x = x * stride;
            domain.push(x);
        }

        Worker {
            config,
            domain,
            evaluations: vec![quartic::transpose(&evaluations, 1)],
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

    pub fn apply_drp(&mut self, alpha: E) {
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

    pub fn query(&self, positions: &[usize]) -> Vec<Vec<QueryResult<E>>> {
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

    pub fn remainder(&self) -> E {
        self.remainder
    }

    fn to_local_positions(&self, positions: &[usize]) -> Vec<usize> {
        let mut local_positions = HashSet::new();
        for &p in positions.iter() {
            if p % self.config.num_partitions == self.config.index {
                local_positions.insert((p - self.config.index) / self.config.num_partitions);
            }
        }
        local_positions.into_iter().collect()
    }

    fn map_positions(&self, positions: &[usize], layer_depth: usize) -> Vec<usize> {
        let mut result = HashSet::new();
        let num_evaluations = self.evaluations[layer_depth].len();
        positions.iter().for_each(|p| {
            result.insert(p % num_evaluations);
        });
        result.into_iter().collect::<Vec<_>>()
    }
}
