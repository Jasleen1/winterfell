// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;

use crypto::hashers::Rp64_256;
use crypto::ElementHasher;
use fractal_proofs::log2;
use math::fields::f64::BaseElement;
use math::FieldElement;
use math::StarkField;

use models::jsnark_arith_parser::R1CSArithReaderParser;

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
    filebase: &str,
    verbose: bool,
) {
    let mut arith_parser = R1CSArithReaderParser::<B>::new().unwrap();
    let arith_file = filebase.to_string() + ".arith";
    let wire_file = filebase.to_string() + ".in";
    arith_parser.parse_arith_file(&arith_file, verbose);
    let r1cs = arith_parser.clone_r1cs();

    // 1. Index this R1CS
    let index_params = IndexParams {
        num_input_variables: 16,
        num_constraints: 16,
        num_non_zero: 8,
    };

    let index_domains = build_index_domains::<B>(index_params.clone());
    let indexed_a = index_matrix::<B>(&r1cs.A, &index_domains);
    let indexed_b = index_matrix::<B>(&r1cs.B, &index_domains);
    let indexed_c = index_matrix::<B>(&r1cs.C, &index_domains);
    // This is the index i.e. the pre-processed data for this r1cs
    let index = Index::new(index_params, indexed_a, indexed_b, indexed_c);

    let (_prover_key, _verifier_key) = generate_prover_and_verifier_keys::<H, B, N>(index).unwrap();

    // TODO: create FractalProver
    let degree_fs = r1cs.clone().get_num_cols();
    let log_size_subgroup_h = log2(degree_fs) + 1u32;
    let log_size_subgroup_k = 2 * log_size_subgroup_h;
    let _size_subgroup_h = 1 << log_size_subgroup_h;
    let _size_subgroup_k = 1 << log_size_subgroup_k;

    // let options: FractalOptions<BaseElement> = FractalOptions::<BaseElement> {
    //     degree_fs,
    //     size_subgroup_h,
    //     size_subgroup_k,
    //     summing_domain,
    //     evaluation_domain,
    //     h_domain,
    //     fri_options,
    //     num_queries,
    // };

    // let mut prover = FractalProver::<BaseElement, BaseElement, Rp64_256>::new(
    //     prover_key,
    //     options,
    //     witness,
    //     variable_assignment,
    //     pub_inputs_byptes
    // );
}
