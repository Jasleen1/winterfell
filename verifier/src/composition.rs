use common::stark::{CompositionCoefficients, ProofContext, StarkProof};
use math::field::{self, add, div, mul, sub};

pub fn compose_registers(
    proof: &StarkProof,
    context: &ProofContext,
    positions: &[usize],
    z: u128,
    cc: &CompositionCoefficients,
) -> Vec<u128> {
    let trace_info = proof.trace_info();
    let lde_root = field::get_root_of_unity(trace_info.lde_domain_size());
    let trace_root = field::get_root_of_unity(trace_info.length());
    let next_z = mul(z, trace_root);

    let trace_at_z1 = proof.get_state_at_z1().to_vec();
    let trace_at_z2 = proof.get_state_at_z2().to_vec();
    let trace_states = proof.trace_states();

    // TODO: this is computed in several paces; consolidate
    let composition_degree = context.deep_composition_degree();
    let incremental_degree = (composition_degree - (proof.trace_info().length() - 2)) as u128;

    let mut result = Vec::with_capacity(trace_states.len());
    for (registers, &position) in trace_states.iter().zip(positions) {
        let x = field::exp(lde_root, position as u128);

        let mut composition = field::ZERO;
        for (i, &value) in registers.iter().enumerate() {
            // compute T1(x) = (T(x) - T(z)) / (x - z)
            let t1 = div(sub(value, trace_at_z1[i]), sub(x, z));
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = add(composition, mul(t1, cc.trace1[i]));

            // compute T2(x) = (T(x) - T(z * g)) / (x - z * g)
            let t2 = div(sub(value, trace_at_z2[i]), sub(x, next_z));
            // multiply it by a pseudo-random coefficient, and combine with result
            composition = add(composition, mul(t2, cc.trace2[i]));
        }

        // raise the degree to match composition degree
        let xp = field::exp(x, incremental_degree);
        let adj_composition = mul(mul(composition, xp), cc.t2_degree);
        composition = add(mul(composition, cc.t1_degree), adj_composition);

        result.push(composition);
    }

    result
}

pub fn compose_constraints(
    proof: &StarkProof,
    t_positions: &[usize],
    c_positions: &[usize],
    z: u128,
    evaluation_at_z: u128,
    cc: &CompositionCoefficients,
) -> Vec<u128> {
    // build constraint evaluation values from the leaves of constraint Merkle proof
    let mut evaluations: Vec<u128> = Vec::with_capacity(t_positions.len());
    let leaves = proof.constraint_proof().values;
    for &position in t_positions.iter() {
        let leaf_idx = c_positions.iter().position(|&v| v == position / 2).unwrap();
        let element_start = (position % 2) * 16;
        let element_bytes = &leaves[leaf_idx][element_start..(element_start + 16)];
        evaluations.push(field::from_bytes(element_bytes));
    }

    let trace_info = proof.trace_info();
    let lde_root = field::get_root_of_unity(trace_info.lde_domain_size());

    // divide out deep point from the evaluations
    let mut result = Vec::with_capacity(evaluations.len());
    for (evaluation, &position) in evaluations.into_iter().zip(t_positions) {
        let x = field::exp(lde_root, position as u128);

        // compute C(x) = (P(x) - P(z)) / (x - z)
        let composition = div(sub(evaluation, evaluation_at_z), sub(x, z));
        // multiply by pseudo-random coefficient for linear combination
        result.push(mul(composition, cc.constraints));
    }

    result
}
