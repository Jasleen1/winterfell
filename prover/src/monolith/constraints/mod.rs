use super::{utils, StarkDomain};
use crypto::{BatchMerkleProof, HashFunction, MerkleTree};
use math::field::FieldElement;

mod assertions;

mod evaluator;
pub use evaluator::ConstraintEvaluator;

mod constraint_poly;
pub use constraint_poly::ConstraintPoly;

mod evaluation_table;
pub use evaluation_table::ConstraintEvaluationTable;

// TODO: re-enable
//#[cfg(test)]
//mod tests;

// PROCEDURES
// ================================================================================================

/// Puts constraint evaluations into a Merkle tree; 2 evaluations per leaf
pub fn build_constraint_tree<E: FieldElement>(
    evaluations: Vec<E>,
    hash_fn: HashFunction,
) -> MerkleTree {
    assert!(
        evaluations.len().is_power_of_two(),
        "number of values must be a power of 2"
    );

    // reinterpret vector of 16-byte values as a vector of 32-byte arrays; this puts
    // pairs of adjacent evaluation values into a single array element
    let mut v = std::mem::ManuallyDrop::new(evaluations);
    let p = v.as_mut_ptr();
    let len = v.len() / 2;
    let cap = v.capacity() / 2;
    let evaluations = unsafe { Vec::from_raw_parts(p as *mut [u8; 32], len, cap) };

    // build Merkle tree out of evaluations
    MerkleTree::new(evaluations, hash_fn)
}

/// Returns constraint evaluations at the specified positions along with Merkle
/// authentication paths from the constraint_tree root to these evaluations.
/// Since evaluations are compressed into a single field element, the are already
/// included in Merkle authentication paths.
pub fn query_constraints(
    constraint_tree: MerkleTree,
    trace_positions: &[usize],
) -> BatchMerkleProof {
    // first, map trace positions to the corresponding positions in the constraint tree;
    // we need to do this because we store 2 constraint evaluations per leaf
    let mut constraint_positions = Vec::with_capacity(trace_positions.len());
    for &position in trace_positions.iter() {
        let cp = position / 2;
        if !constraint_positions.contains(&cp) {
            constraint_positions.push(cp);
        }
    }

    // build Merkle authentication paths to the leaves specified by constraint positions
    constraint_tree.prove_batch(&constraint_positions)
}
