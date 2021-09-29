use crypto::{ElementHasher, Hasher};
use math::{
    StarkField
};

use fri::{DefaultVerifierChannel, FriVerifier, VerifierChannel, VerifierError};

use fractal_proofs::{FieldElement, SumcheckProof};
use fractal_utils::*;

// pub struct SumcheckVerifier<E>  {
//     context: VerifierContext,
//     proof: SumcheckProof,
// }

pub fn verify_sumcheck_proof<B: StarkField, E: FieldElement<BaseField = B>, H: ElementHasher<BaseField = B>>(
    proof: SumcheckProof<B, E, H>,
) -> Result<(), VerifierError> {
    let g_channel =
        DefaultVerifierChannel::<E, H>::new(proof.g_proof, proof.g_commitments, proof.g_max_degree, proof.options.folding_factor())?;
    // let g_context = VerifierContext::new(
    //     proof.num_evaluations,
    //     proof.g_max_degree,
    //     g_channel.num_fri_partitions(),
    //     proof.options.clone(),
    // );
    let g_verifier = FriVerifier::<B, E, DefaultVerifierChannel<E, H>, H>::new(
        &mut g_channel, g_channel.public_coin(),
        proof.options.clone(), proof.g_max_degree
    );
    let g_queried_evals = proof.g_queried_evals;
    let g_verifies = g_verifier.verify(
        &g_channel,
        &g_queried_evals,
        &proof.queried_positions,
    )?;
    if g_verifies.is_ok() {
        let e_channel =
            DefaultVerifierChannel::new(proof.e_proof, proof.e_commitments, &proof.options);
        // let e_context = VerifierContext::new(
        //     proof.num_evaluations,
        //     proof.e_max_degree,
        //     e_channel.num_fri_partitions(),
        //     proof.options.clone(),
        // );
        let e_verifier = FriVerifier::<B, E, DefaultVerifierChannel<E, H>, H>::new(
            e_channel, e_channel.public_coin(),
            proof.options.clone(), proof.e_max_degree
        );
        let e_queried_evals = proof.e_queried_evals;
        e_verifier.verify(
            &e_channel,
            &e_queried_evals,
            &proof.queried_positions,
        )
    } else {
        g_verifies
    }
}
