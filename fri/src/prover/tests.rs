use crate::{
    verifier, DefaultProverChannel, DefaultVerifierChannel, FriOptions, FriProof, VerifierChannel,
    VerifierContext,
};
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

// TEST UTILS
// ================================================================================================

pub fn build_prover_channel(trace_length: usize, options: &FriOptions) -> DefaultProverChannel {
    DefaultProverChannel::new(options.clone(), trace_length * options.blowup_factor(), 32)
}

pub fn build_lde_domain(trace_length: usize, lde_blowup: usize) -> Vec<BaseElement> {
    let domain_size = trace_length * lde_blowup;
    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    BaseElement::get_power_series(g, domain_size)
}

pub fn build_evaluations(
    trace_length: usize,
    lde_blowup: usize,
    ce_blowup: usize,
) -> Vec<BaseElement> {
    let len = (trace_length * ce_blowup) as u128;
    let mut p = (0..len).map(BaseElement::new).collect::<Vec<_>>();
    let domain_size = trace_length * lde_blowup;
    p.resize(domain_size, BaseElement::ZERO);

    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let twiddles = fft::get_twiddles(g, domain_size);

    fft::evaluate_poly(&mut p, &twiddles);
    p
}

pub fn verify_proof(
    proof: FriProof,
    commitments: Vec<[u8; 32]>,
    evaluations: &[BaseElement],
    max_degree: usize,
    positions: &[usize],
    options: &FriOptions,
) -> Result<bool, String> {
    let channel = DefaultVerifierChannel::new(proof, commitments, options);
    let context = VerifierContext::new(
        evaluations.len(),
        max_degree,
        channel.num_fri_partitions(),
        options.clone(),
    );
    let queried_evaluations = positions
        .iter()
        .map(|&p| evaluations[p])
        .collect::<Vec<_>>();
    verifier::verify(&context, &channel, &queried_evaluations, &positions)
}
