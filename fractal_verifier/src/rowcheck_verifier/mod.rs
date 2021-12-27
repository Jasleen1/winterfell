use crypto::{ElementHasher, RandomCoin};
use math::StarkField;

use fri::{DefaultVerifierChannel, FriVerifier, VerifierError};

use fractal_proofs::{FieldElement, RowcheckProof, FOLDING_FACTOR};
use utils::Serializable;

pub fn verify_rowcheck_proof<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher<BaseField = B>,
>(
    proof: RowcheckProof<B, E, H>,
) -> Result<(), VerifierError> {
    let mut public_coin_seed = Vec::new();
    proof.write_into(&mut public_coin_seed);
    let mut public_coin = RandomCoin::new(&public_coin_seed);

    let mut channel = DefaultVerifierChannel::new(
        proof.s_proof,
        proof.s_commitments,
        proof.s_max_degree,
        FOLDING_FACTOR,
    )
    .map_err(VerifierError::DeserializationErr)?;
    let s_queried_evals = proof.s_queried_evals;
    let fri_verifier = FriVerifier::<B, E, DefaultVerifierChannel<E, H>, H>::new(
        &mut channel,
        &mut public_coin,
        proof.options.clone(),
        proof.s_max_degree,
    )?;
    fri_verifier.verify(&mut channel, &s_queried_evals, &proof.queried_positions)
}
