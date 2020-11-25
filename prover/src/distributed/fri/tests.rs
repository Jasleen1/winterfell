use crate::{channel::ProverChannel, tests::build_proof_context};
use common::{fri_utils, ComputationContext};
use crypto::{hash::blake3, MerkleTree};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
    quartic,
};

use super::Prover;

// TESTS
// ================================================================================================

#[test]
fn fri_prover() {
    let trace_length = 32;
    let ce_blowup = 2;
    let lde_blowup = 8;
    let context = build_proof_context(trace_length, ce_blowup, lde_blowup);
    let evaluations = build_evaluations(trace_length, ce_blowup, lde_blowup);
    let mut prover = Prover::new(&context, &evaluations);

    let mut channel = build_prover_channel(&context);
    prover.build_layers(&mut channel);

    let result = prover.build_proof(&[65, 123]);

    let proof = &result.layers[0];

    /*
    println!("values: -------------");
    for value in proof.values.iter() {
        println!("{}", hex::encode(value));
    }

    for (i, nodes) in proof.paths.iter().enumerate() {
        println!("{} -------------", i);
        for n in nodes.iter() {
            println!("{}", hex::encode(n));
        }
    }

    println!("=====================");

    let ap = get_augmented_positions(&[65, 123], evaluations.len(), prover.num_partitions());
    let layer_one_tree = build_layer_one_tree(&evaluations, prover.num_partitions());

    let proof = layer_one_tree.prove_batch(&ap);

    println!("values: -------------");
    for value in proof.values.iter() {
        println!("{}", hex::encode(value));
    }

    for (i, nodes) in proof.nodes.iter().enumerate() {
        println!("{} -------------", i);
        for n in nodes.iter() {
            println!("{}", hex::encode(n));
        }
    }
    */
    /*
    let proof = layer_one_tree.prove(ap[0]);
    for node in proof.iter() {
        println!("{}", hex::encode(node));
    }
    println!("-------------");
    let proof = layer_one_tree.prove(ap[1]);
    for node in proof.iter() {
        println!("{}", hex::encode(node));
    }
    println!("-------------");

    for r in result.iter() {
        for q in r.iter() {
            println!("index: {}", q.index);
            for node in q.path.iter() {
                println!("{}", hex::encode(node));
            }
        }
    }
    */

    assert!(false);
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_evaluations(trace_length: usize, ce_blowup: usize, lde_blowup: usize) -> Vec<BaseElement> {
    let len = (trace_length * ce_blowup) as u128;
    let mut p = (0..len).map(BaseElement::new).collect::<Vec<_>>();
    let domain_size = trace_length * lde_blowup;
    p.resize(domain_size, BaseElement::ZERO);

    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let twiddles = fft::get_twiddles(g, domain_size);

    fft::evaluate_poly(&mut p, &twiddles, true);
    p
}

fn build_prover_channel(context: &ComputationContext) -> ProverChannel {
    let channel = ProverChannel::new(context);

    channel
}

fn build_layer_one_tree(evaluations: &[BaseElement], num_partitions: usize) -> MerkleTree {
    let evaluations = quartic::transpose(&evaluations, 1);
    let partition_size = evaluations.len() / num_partitions;

    let hashed_evaluations = fri_utils::hash_values(&evaluations, blake3);
    let mut result = Vec::new();
    for i in 0..num_partitions {
        for j in 0..partition_size {
            result.push(hashed_evaluations[i + j * num_partitions]);
        }
    }

    MerkleTree::new(result, blake3)
}

fn get_augmented_positions(
    positions: &[usize],
    num_evaluations: usize,
    num_partitions: usize,
) -> Vec<usize> {
    let local_bits = (num_evaluations / 4).trailing_zeros() - num_partitions.trailing_zeros();
    let positions = fri_utils::get_augmented_positions(&positions, num_evaluations);
    let mut result = Vec::new();
    for &p in positions.iter() {
        let p_idx = p % num_partitions;
        let loc_p = (p - p_idx) / num_partitions;
        result.push((p_idx << local_bits) | loc_p);
    }
    result
}
