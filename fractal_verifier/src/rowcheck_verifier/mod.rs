use math::{
    StarkField
};

use fri::{
    DefaultVerifierChannel, VerifierChannel, VerifierError,
};

use fractal_proofs::RowcheckProof;


pub fn verify_rowcheck_proof<E: StarkField>(
    _proof: RowcheckProof<E>,
) -> Result<(), VerifierError> {
    let channel =
        DefaultVerifierChannel::new(_proof.s_proof, _proof.s_commitments, &_proof.options);
    let context = VerifierContext::new(
        _proof.num_evaluations,
        _proof.s_max_degree,
        channel.num_fri_partitions(),
        _proof.options.clone(),
    );
    let s_queried_evals = _proof.s_queried_evals;
    verify(
        &context,
        &channel,
        &s_queried_evals,
        &_proof.queried_positions,
    )
}
