// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;

use crypto::ElementHasher;
use crypto::hashers::Rp64_256;
use math::FieldElement;
use math::StarkField;
use math::fields::f64::BaseElement;

use models::arith_parser::R1CSArithReaderParser;

use fractal_indexer::{
    index::{build_index_domains, Index, IndexParams},
    indexed_matrix::index_matrix,
    snark_keys::*,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut input_file = "./src/sample.arith";
    if args.len() > 1 {
         input_file = &args[1];
    }

    let verbose = true;
    if verbose {
        println!("Parse file {}", input_file);
    }

    // call orchestrate_r1cs_example
    orchestrate_r1cs_example::<BaseElement, BaseElement, Rp64_256, 16>(input_file, verbose);
}

pub(crate) fn orchestrate_r1cs_example<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
    const N: usize,
>(
    input_file: &str,
    verbose: bool,
) {
    let mut arith_parser = R1CSArithReaderParser::<B>::new().unwrap();
    arith_parser.parse_file(input_file, verbose);
    let r1cs = arith_parser.r1cs_instance;

    // 1. Index this R1CS
    let index_params = IndexParams { num_input_variables: 16, num_constraints: 16, num_non_zero: 8 };

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
