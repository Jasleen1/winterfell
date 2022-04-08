use crypto::{ElementHasher, RandomCoin};
use fractal_proofs::{FieldElement, FractalProof, StarkField};
use fri::VerifierError;
use fractal_indexer::snark_keys::*;

use crate::{lincheck_verifier::verify_lincheck_proof, rowcheck_verifier::verify_rowcheck_proof};

pub fn verify_fractal_proof<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher<BaseField = B>,
>(
    verifier_key: VerifierKey<H, B>,
    proof: FractalProof<B, E, H>,
    pub_inputs_bytes: Vec<u8>,
) -> Result<(), VerifierError> {
    let mut public_coin = RandomCoin::<_, H>::new(&pub_inputs_bytes);
    let expected_alpha: B = public_coin.draw().expect("failed to draw OOD point");
    
    verify_rowcheck_proof(verifier_key, proof.rowcheck_proof)?;
    println!("Rowcheck verified");
    verify_lincheck_proof(verifier_key, proof.lincheck_a, expected_alpha)?;
    println!("Lincheck a verified");
    verify_lincheck_proof(verifier_key, proof.lincheck_b, expected_alpha)?;
    println!("Lincheck b verified");
    verify_lincheck_proof(verifier_key, proof.lincheck_c, expected_alpha)?;
    println!("Lincheck c verified");
    
    Ok(())
}
