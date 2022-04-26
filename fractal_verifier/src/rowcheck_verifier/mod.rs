use crypto::{ElementHasher, RandomCoin};
use fractal_indexer::snark_keys::VerifierKey;
use math::StarkField;

use fri::{DefaultVerifierChannel, FriVerifier, VerifierError};

use fractal_proofs::{FieldElement, RowcheckProof, FOLDING_FACTOR};
use utils::Serializable;

pub fn verify_rowcheck_proof<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher<BaseField = B>,
>(
    verifier_key: &VerifierKey<H, B>,
    proof: RowcheckProof<B, E, H>,
    // Change to include public seed
) -> Result<(), VerifierError> {
    // let mut public_coin_seed = Vec::new();
    // proof.write_into(&mut public_coin_seed);
    let mut public_coin = RandomCoin::new(&[]);

    let mut channel = DefaultVerifierChannel::new(
        proof.s_proof,
        proof.s_commitments,
        proof.num_evaluations,
        proof.options.folding_factor(),
    )
    .map_err(VerifierError::DeserializationErr)?;
    let s_queried_evals = proof.s_queried_evals;
    let fri_verifier = FriVerifier::<B, E, DefaultVerifierChannel<E, H>, H>::new(
        &mut channel,
        &mut public_coin,
        proof.options.clone(),
        verifier_key.params.max_degree - 1,
    )?;
    println!("s max deg in rowcheck = {}", verifier_key.params.max_degree - 1);
    fri_verifier.verify(&mut channel, &s_queried_evals, &proof.queried_positions)
}
