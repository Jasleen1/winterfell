use crate::channel::ProverChannel;
use common::{ComputationContext, PublicCoin};
use crypto::{HashFunction, MerkleTree};
use math::field::{BaseElement, FieldElement};

mod worker;
use worker::{QueryResult, Worker, WorkerConfig};

mod tests;

// CONSTANTS
// ================================================================================================
const FOLDING_FACTOR: usize = ComputationContext::FRI_FOLDING_FACTOR;

// TYPES AND INTERFACES
// ================================================================================================

pub struct Prover<E: FieldElement + From<BaseElement>> {
    workers: Vec<Worker<E>>,
    hash_fn: HashFunction,
    domain_size: usize,
    num_layers: usize,
    layer_trees: Vec<MerkleTree>,
}

// FRI PROVER IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>> Prover<E> {
    pub fn new(context: &ComputationContext, evaluations: &[E]) -> Self {
        let hash_fn = context.options().hash_fn();
        let domain_size = evaluations.len();

        // break up evaluations into partitions to be sent to individual workers; we set the
        // number of partitions to be equal to the length of FRI base layer (the remainder)
        let num_layers = context.num_fri_layers();
        let num_partitions = context.fri_remainder_length();
        let partitions = partition(&evaluations, num_partitions);

        // create workers and assign partitions to them; in the real distributed context,
        // partitions will be the output of distributed FFT computation and will already be
        // in memory of individual workers.
        let mut workers = Vec::new();
        for (i, partition) in partitions.into_iter().enumerate() {
            let config = WorkerConfig {
                num_partitions,
                index: i,
                domain_size,
                hash_fn,
            };
            workers.push(Worker::new(config, &partition))
        }

        Prover {
            workers,
            domain_size,
            num_layers,
            hash_fn,
            layer_trees: Vec::new(),
        }
    }

    pub fn num_partitions(&self) -> usize {
        self.workers.len()
    }

    pub fn build_layers(&mut self, channel: &mut ProverChannel) {
        for layer_depth in 0..self.num_layers {
            // commit to the current layer across all workers; we do this by first having each
            // worker commit to their current layers, and then building a Merkle tree from
            // worker commitments. This has very low communication overhead since each work
            // sends just the root of their internal Merkle tree back to the prover.
            let worker_commitments = self.workers.iter_mut().map(|w| w.commit()).collect();
            let layer_tree = MerkleTree::new(worker_commitments, self.hash_fn);
            channel.commit_fri_layer(*layer_tree.root());
            self.layer_trees.push(layer_tree);

            // draw random coefficient from the channel and use it to perform degree-preserving
            // projections in each worker.
            let alpha = channel.draw_fri_point::<E>(layer_depth);
            self.workers.iter_mut().for_each(|w| w.apply_drp(alpha));
        }
    }

    pub fn build_proof(&self, positions: &[usize]) -> Vec<Vec<QueryResult<E>>> {
        let mut queries = (0..self.num_layers).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut remainder = Vec::new();
        for (worker_idx, worker) in self.workers.iter().enumerate() {
            let r = worker.query(positions);
            for (layer_depth, layer_results) in r.into_iter().enumerate() {
                let path_end = self.layer_trees[layer_depth].prove(worker_idx);
                for mut query in layer_results.into_iter() {
                    if query.path.is_empty() {
                        query.path = path_end.clone();
                    } else {
                        query.path.extend_from_slice(&path_end[1..]);
                    }
                    query.index = self.to_global_position(worker_idx, layer_depth, query.index);
                    queries[layer_depth].push(query);
                }
            }
            remainder.push(worker.remainder());
        }
        queries
    }

    fn to_global_position(
        &self,
        worker_idx: usize,
        layer_depth: usize,
        local_position: usize,
    ) -> usize {
        let num_evaluations =
            self.domain_size / usize::pow(FOLDING_FACTOR, (layer_depth + 1) as u32);
        let local_bits = num_evaluations.trailing_zeros() - self.workers.len().trailing_zeros();
        (worker_idx << local_bits) | local_position
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn partition<E: FieldElement>(evaluations: &[E], num_partitions: usize) -> Vec<Vec<E>> {
    let mut result = Vec::new();
    for _ in 0..num_partitions {
        result.push(Vec::new());
    }

    for i in 0..evaluations.len() {
        result[i % num_partitions].push(evaluations[i]);
    }

    result
}
