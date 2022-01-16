// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crypto::ElementHasher;
use fractal_indexer::{
    index::{build_index_domains, Index, IndexParams},
    indexed_matrix::index_matrix,
    snark_keys::*,
};
use fractal_proofs::FieldElement;

use math::StarkField;

use crate::arith_parser_example::reading_arith;

pub(crate) fn r1cs_end_to_end_example<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
    const N: usize,
>(
    input_file: &str,
    index_params: IndexParams,
) {
    let r1cs = reading_arith::<B>(input_file, false);

    // 1. Index this R1CS
    let index_domains = build_index_domains::<B>(index_params.clone());
    let indexed_a = index_matrix::<B>(r1cs.A, index_domains.clone());
    let indexed_b = index_matrix::<B>(r1cs.B, index_domains.clone());
    let indexed_c = index_matrix::<B>(r1cs.C, index_domains.clone());
    // This is the index i.e. the pre-processed data for this r1cs
    let index = Index::new(index_params, indexed_a, indexed_b, indexed_c);

    let (_prover_key, _verifier_key) = generate_prover_and_verifier_keys::<H, B, N>(index).unwrap();

    // TODO
    // NEXT STEPS
    // 2. Get the prover arguments
    // let mut prover = FractalProver::<B, E, H>(
    //     prover_key,

    // )
}
