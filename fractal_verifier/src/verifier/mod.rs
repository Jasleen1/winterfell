use crypto::ElementHasher;
use fractal_proofs::{FieldElement, FractalProof, StarkField};
use fri::VerifierError;

use crate::{lincheck_verifier::verify_lincheck_proof, rowcheck_verifier::verify_rowcheck_proof};

pub fn verify_fractal_proof<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher<BaseField = B>,
>(
    proof: FractalProof<B, E, H>,
) -> Result<(), VerifierError> {
    verify_rowcheck_proof(proof.rowcheck_proof)?;
    verify_lincheck_proof(proof.lincheck_a)?;
    verify_lincheck_proof(proof.lincheck_b)?;
    verify_lincheck_proof(proof.lincheck_c)?;
    Ok(())
}
