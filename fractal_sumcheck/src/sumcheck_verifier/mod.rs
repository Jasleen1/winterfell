use math::{
    field::{BaseElement, FieldElement}
};

use winter-fri::{
    verify, DefaultVerifierChannel, VerifierChannel, VerifierContext, VerifierError,
};

use fractal_proofs::SumcheckProof;

// pub struct SumcheckVerifier<E>  {
//     context: VerifierContext,
//     proof: SumcheckProof,
// }

pub fn verify_sumcheck_proof<E: FieldElement + From<BaseElement>>(
    _proof: SumcheckProof<E>,
) -> Result<(), VerifierError> {
    let g_channel =
        DefaultVerifierChannel::new(_proof.g_proof, _proof.g_commitments, &_proof.options);
    let g_context = VerifierContext::new(
        _proof.num_evaluations,
        _proof.g_max_degree,
        g_channel.num_fri_partitions(),
        _proof.options.clone(),
    );
    let g_queried_evals = _proof.g_queried_evals;
    let g_verifies = verify(
        &g_context,
        &g_channel,
        &g_queried_evals,
        &_proof.queried_positions,
    );
    if g_verifies.is_ok() {
        let e_channel =
            DefaultVerifierChannel::new(_proof.e_proof, _proof.e_commitments, &_proof.options);
        let e_context = VerifierContext::new(
            _proof.num_evaluations,
            _proof.e_max_degree,
            e_channel.num_fri_partitions(),
            _proof.options.clone(),
        );
        let _e_queried_evals = _proof.e_queried_evals;
        verify(
            &e_context,
            &e_channel,
            &g_queried_evals,
            &_proof.queried_positions,
        )
    } else {
        g_verifies
    }
}
