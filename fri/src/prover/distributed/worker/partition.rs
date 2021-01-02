use super::super::QueryResult;
use crate::{folding::quartic, utils::hash_values};
use crypto::{HashFunction, MerkleTree};
use math::field::{BaseElement, FieldElement, StarkField};
use std::{collections::HashSet, sync::Arc};

// CONSTANTS
// ================================================================================================
// TODO: get from somewhere
const FOLDING_FACTOR: usize = 4;

// TYPES AND INTERFACES
// ================================================================================================

pub struct Partition {
    domain: Vec<BaseElement>,
    evaluations: Vec<Vec<[BaseElement; FOLDING_FACTOR]>>,
    remainder: BaseElement,
    layer_trees: Vec<MerkleTree>,
    partition_idx: usize,
    num_partitions: usize,
}

// PARTITION IMPLEMENTATION
// ================================================================================================

impl Partition {
    pub fn new(
        partition_idx: usize,
        num_partitions: usize,
        evaluations: Arc<Vec<BaseElement>>,
    ) -> Self {
        assert!(
            partition_idx < num_partitions,
            "partition index out of bounds"
        );
        // build a domain for the top layer of evaluations
        let global_domain_size = evaluations.len();
        let g = BaseElement::get_root_of_unity(global_domain_size.trailing_zeros());
        let stride = g.exp(num_partitions as u128);
        let mut x = g.exp(partition_idx as u128);

        let domain_size = global_domain_size / num_partitions;
        let mut domain = Vec::with_capacity(domain_size);
        domain.push(x);
        for _ in 1..domain_size {
            x = x * stride;
            domain.push(x);
        }

        // make a copy of evaluations relevant for this partition
        let mut partition = Vec::new();
        for i in (partition_idx..evaluations.len()).step_by(num_partitions) {
            partition.push(evaluations[i]);
        }
        let evaluations = vec![quartic::transpose(&partition, 1)];

        Partition {
            domain,
            evaluations,
            remainder: BaseElement::ZERO,
            layer_trees: Vec::new(),
            partition_idx,
            num_partitions,
        }
    }

    pub fn index(&self) -> usize {
        self.partition_idx
    }

    pub fn remainder(&self) -> BaseElement {
        self.remainder
    }

    pub fn current_layer(&self) -> usize {
        self.evaluations.len() - 1
    }

    // FRI PROCEDURES
    // --------------------------------------------------------------------------------------------

    pub fn commit_layer(&mut self, hash_fn: HashFunction) -> [u8; 32] {
        let evaluations = &self.evaluations[self.current_layer()];
        let hashed_evaluations = hash_values(&evaluations, hash_fn);
        if hashed_evaluations.len() == 1 {
            hashed_evaluations[0]
        } else {
            let evaluation_tree = MerkleTree::new(hashed_evaluations, hash_fn);
            let root = *evaluation_tree.root();
            self.layer_trees.push(evaluation_tree);
            root
        }
    }

    pub fn apply_drp(&mut self, alpha: BaseElement) {
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

    pub fn query(&self, positions: &[usize]) -> Vec<Vec<QueryResult>> {
        // filter out positions which don't belong to this partition, and if there is
        // nothing to query, return with empty vector
        let mut positions = self.to_local_positions(positions);
        if positions.is_empty() {
            return vec![];
        }

        let mut result = Vec::new();
        for (layer_depth, evaluations) in self.evaluations.iter().enumerate() {
            positions = self.fold_positions(&positions, layer_depth);
            let mut layer_result = Vec::new();
            for &position in positions.iter() {
                let path = if layer_depth < self.layer_trees.len() {
                    self.layer_trees[layer_depth].prove(position)
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

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn to_local_positions(&self, positions: &[usize]) -> Vec<usize> {
        let mut local_positions = HashSet::new();
        for &p in positions.iter() {
            if p % self.num_partitions == self.partition_idx {
                local_positions.insert((p - self.partition_idx) / self.num_partitions);
            }
        }
        local_positions.into_iter().collect()
    }

    fn fold_positions(&self, positions: &[usize], layer_depth: usize) -> Vec<usize> {
        let mut result = HashSet::new();
        let num_evaluations = self.evaluations[layer_depth].len();
        positions.iter().for_each(|p| {
            result.insert(p % num_evaluations);
        });
        result.into_iter().collect::<Vec<_>>()
    }
}
