pub trait PolyCommit {
    type Channel;
    type Options;
    type Queries;
    pub fn new(options: Self::Options) -> Self;
    // pub fn commit(&mut self, poly: Polynomial, channel: &mut Self::Channel, domain: Vec<Self::Domain>) -> ;
    // pub fn commit(&mut self, poly: Polynomial, channel: &mut Self::Channel);
    // pub fn prove(&mut self, channel: &mut Self::Channel, values: Vec<Self::Queries>, )
}
/*


/// R1CSIndexer::ModelOfComputation = R1CSInstance; <-- 3 matrices

/// ProverKey VerifierKey are traits
pub trait Indexer {
    type Options; // Field, PolyCommit, etc.
    type PolyCommitment: PolyCommit;
    type ModelOfComputation;
    type PK: ProverKey;
    type VK: VerifierKey;
    pub fn index_comp(computation: ModelOfComputation) -> (PK, VK);
}


pub trait Proof {
    type Options;
}
/// (f_1, ..., f_n) poly
/// F((f_i)) = (0, ..., 0)?
pub trait PolynomialRelationships {
    fn prove_relationships(polys: Vec<Polynomial>) -> Vec<Proof> {
        // and-ing a bunch of bools about elts of polys
    }
}

pub trait Prover {
    type Options;
    type ModelOfComputation: ModelOfComp;
    type Key: ProverKey;
    type PolyCommitment: PolyCommit;
    type Witness: SnarkWitness;
    type Input: SnarkInput;
}
*/
