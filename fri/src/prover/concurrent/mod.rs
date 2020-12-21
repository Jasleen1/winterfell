use crate::{folding::quartic, utils, FriOptions, FriProof, FriProofLayer, ProverChannel};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::field::{BaseElement, FieldElement};
use std::marker::PhantomData;

mod worker;
use worker::{QueryResult, Worker, WorkerConfig};

mod tests;

// CONSTANTS
// ================================================================================================
const FOLDING_FACTOR: usize = crate::options::FOLDING_FACTOR;

// TYPES AND INTERFACES
// ================================================================================================

pub struct Prover<E: FieldElement + From<BaseElement>, C: ProverChannel> {
    workers: Vec<Worker<E>>,
    hash_fn: HashFunction,
    domain_size: usize,
    num_layers: usize,
    layer_trees: Vec<MerkleTree>,
    _marker: PhantomData<C>,
}

// FRI PROVER IMPLEMENTATION
// ================================================================================================

impl<E: FieldElement + From<BaseElement>, C: ProverChannel> Prover<E, C> {
    /// Returns a new FRI prover for the specified context and evaluations. In the actual
    /// distributed implementation evaluations will probably be provided via references to
    /// distributed data structure.
    /// TODO: this is not a concurrent implementation - it needs to be converted into one
    pub fn new(options: &FriOptions, evaluations: &[E]) -> Self {
        let hash_fn = options.hash_fn();
        let domain_size = evaluations.len();

        // break up evaluations into partitions to be sent to individual workers; we set the
        // number of partitions to be equal to the length of FRI base layer (the remainder)
        let num_layers = options.num_fri_layers(domain_size);
        let num_partitions = options.fri_remainder_length(domain_size);
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
            _marker: PhantomData,
        }
    }

    /// Returns number of partitions into which the original evaluations were broken into.
    pub fn num_partitions(&self) -> usize {
        self.workers.len()
    }

    /// Executes commit phase of FRI protocol which recursively applies a degree-respecting projection
    /// to evaluations of some function F over a larger domain. At each layer of recursion the
    /// current evaluations are committed to using a Merkle tree, and the root of this tree is used
    /// to derive randomness for the subsequent application of degree-respecting projection.
    pub fn build_layers(&mut self, channel: &mut C) {
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
            let alpha = channel.draw_fri_alpha::<E>(layer_depth);
            self.workers.iter_mut().for_each(|w| w.apply_drp(alpha));
        }

        // commit to the remainder
        let remainder = self
            .workers
            .iter()
            .map(|w| w.remainder())
            .collect::<Vec<_>>();
        let remainder = quartic::transpose(&remainder, 1);
        let remainder_hashes = utils::hash_values(&remainder, self.hash_fn);
        let remainder_tree = MerkleTree::new(remainder_hashes, self.hash_fn);
        channel.commit_fri_layer(*remainder_tree.root());
    }

    /// Executes query phase of FRI protocol. For each of the provided `positions`, corresponding
    /// evaluations from each of the layers are recorded into the proof together with Merkle
    /// authentication paths from the root of layer commitment trees.
    pub fn build_proof(&self, positions: &[usize]) -> FriProof {
        let mut queries = (0..self.num_layers).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut remainder = Vec::new();

        // iterate over all workers to collect query and remainder info from them
        for (worker_idx, worker) in self.workers.iter().enumerate() {
            // query the worker; if positions are applicable to this worker then a set
            // of query results will be returned; otherwise and we get an empty vector back
            let worker_results = worker.query(positions);
            for (layer_depth, layer_results) in worker_results.into_iter().enumerate() {
                // queries from each worker contain only starting segments of Merkle
                // authentication paths against virtual Merkle tree of all evaluations -
                // so, we need to complete them with the segment held by the prover.
                let path_end = self.layer_trees[layer_depth].prove(worker_idx);
                for mut query in layer_results.into_iter() {
                    // empty path from the worker indicates that we reached the last layer
                    // where there was only a single leaf; in this case, we use the entire
                    // path from the prover.
                    if query.path.is_empty() {
                        query.path = path_end.clone();
                    } else {
                        // if we are not at the last layer, we need to skip the first node of
                        // the prover's path because it is implied by the path sent by the worker
                        query.path.extend_from_slice(&path_end[1..]);
                    }
                    // we also translate query index local to the worker into index applicable to
                    // the entire virtual tree
                    query.index = self.to_global_index(worker_idx, layer_depth, query.index);
                    queries[layer_depth].push(query);
                }
            }
            remainder.push(worker.remainder());
        }

        // build FRI layers from the queries
        let layers = queries
            .into_iter()
            .map(|mut q| build_fri_layer(&mut q))
            .collect();

        FriProof {
            layers,
            rem_values: E::write_into_vec(&remainder),
            partitioned: true,
        }
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn to_global_index(&self, worker_idx: usize, layer_depth: usize, local_idx: usize) -> usize {
        let num_evaluations =
            self.domain_size / usize::pow(FOLDING_FACTOR, (layer_depth + 1) as u32);
        let local_bits = num_evaluations.trailing_zeros() - self.workers.len().trailing_zeros();
        (worker_idx << local_bits) | local_idx
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

fn build_fri_layer<E: FieldElement>(queries: &mut [QueryResult<E>]) -> FriProofLayer {
    queries.sort_by_key(|q| q.index);

    let mut indexes = Vec::new();
    let mut paths = Vec::new();
    let mut values = Vec::new();

    for query in queries.iter() {
        indexes.push(query.index);
        paths.push(query.path.clone());
        values.push(E::write_into_vec(&query.value));
    }

    let batch_proof = BatchMerkleProof::from_paths(&paths, &indexes);

    FriProofLayer {
        values,
        paths: batch_proof.nodes,
        depth: batch_proof.depth,
    }
}
