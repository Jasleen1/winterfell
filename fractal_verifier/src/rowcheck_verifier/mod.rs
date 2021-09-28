use math::{
    StarkField
};

use fri::{DefaultVerifierChannel, FriVerifier, VerifierChannel, VerifierError};

use fractal_proofs::RowcheckProof;


pub fn verify_rowcheck_proof<E: StarkField>(
    proof: RowcheckProof<E>,
) -> Result<(), VerifierError> {
    let channel =
        DefaultVerifierChannel::new(proof.s_proof, proof.s_commitments, &proof.options, 4);
    let context = VerifierContext::new(
        proof.num_evaluations,
        proof.s_max_degree,
        channel.num_fri_partitions(),
        proof.options.clone(),
    );
    let s_queried_evals = proof.s_queried_evals;
    let fri_verifier = FriVerifier::new(&mut channel, &mut )
    fri_verifier::verify(
        &channel,
        &s_queried_evals,
        &proof.queried_positions,
    )
}
