// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::cmp::max;

use fractal_proofs::FriOptions;
use fractal_prover::FractalOptions;
use fractal_prover::prover::FractalProver;
use structopt::StructOpt;

use crypto::hashers::Rp64_256;
use crypto::ElementHasher;
use fractal_proofs::log2;
use math::fields::f64::BaseElement;
use math::FieldElement;
use math::StarkField;
use math::utils;

use models::jsnark_arith_parser::JsnarkArithReaderParser;
use models::jsnark_wire_parser::JsnarkWireReaderParser;

use fractal_indexer::{
    index::{build_index_domains, Index, IndexParams},
    indexed_matrix::index_matrix,
    snark_keys::*,
};

fn main() {
    let options = ExampleOptions::from_args();
    if options.verbose {
        println!("Arith file {}, wire value file {}", options.arith_file, options.wires_file);
    }

    // call orchestrate_r1cs_example
    // orchestrate_r1cs_example::<BaseElement, BaseElement, Rp64_256, 16>(
    //     &options.arith_file,
    //     &options.wires_file,
    //     options.verbose);
}

pub(crate) fn orchestrate_r1cs_example<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B> + Clone,
    const N: usize,
>(
    arith_file: &str,
    wire_file: &str,
    verbose: bool,
) {
    let mut arith_parser = JsnarkArithReaderParser::<B>::new().unwrap();
    arith_parser.parse_arith_file(&arith_file, verbose);
    let r1cs = arith_parser.clone_r1cs();

    let mut wires_parser = JsnarkWireReaderParser::<B>::new().unwrap();
    wires_parser.parse_wire_file(&wire_file, verbose);
    let wires = wires_parser.wires;
    // 0. Compute num_non_zero by counting max(number of non-zero elts across A, B, C).
    
    let num_input_variables = r1cs.clone().num_cols();
    let num_constraints = r1cs.clone().num_rows();
    let num_non_zero = max(max(r1cs.A.l0_norm(), r1cs.B.l0_norm()), r1cs.C.l0_norm());
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
    let index = Index::new(index_params.clone(), indexed_a, indexed_b, indexed_c);

    let (prover_key, _verifier_key) = generate_prover_and_verifier_keys::<H, B, N>(index).unwrap();

    // TODO: create FractalProver
    let degree_fs = r1cs.clone().num_rows();
    let log_size_subgroup_h = max(log2(r1cs.clone().num_cols()), log2(r1cs.clone().num_rows()));
    let log_size_subgroup_k = log2(index_params.num_non_zero) + 1u32;
    let size_subgroup_h = 1 << log_size_subgroup_h;
    let size_subgroup_k = 1 << log_size_subgroup_k;
    // To get eval domain, just compute size L as in paper, 
    // to get evals for L, using fft.evaluate

    let summing_domain = utils::get_power_series(index_domains.k_field_base, index_domains.k_field_len);
    let evaluation_domain = utils::get_power_series(index_domains.l_field_base, index_domains.l_field_len);
    let h_domain = index_domains.h_field;
    let lde_blowup = 8;
    let num_queries = 32;
    let fri_options = FriOptions::new(lde_blowup, 4, 256);
    let options: FractalOptions<B> = FractalOptions::<B> {
        degree_fs,
        size_subgroup_h,
        size_subgroup_k,
        summing_domain,
        evaluation_domain,
        h_domain,
        fri_options,
        num_queries,
    };
    let pub_inputs_bytes = vec![0u8];
    let mut prover = FractalProver::<B, E, H>::new(
        prover_key,
        options,
        vec![],
        wires,
        pub_inputs_bytes
    );
}

#[derive(StructOpt, Debug)]
#[structopt(name = "jsnark-parser", about = "Jsnark file parsing")]
struct ExampleOptions {
    /// Jsnark .arith file to parse.
    #[structopt(short = "a", long = "arith_file", default_value = "sample.arith")]
    arith_file: String,

    /// Jsnark .in or .wires file to parse.
    #[structopt(short = "w", long = "wire_file", default_value = "sample.wires")]
    wires_file: String,

    /// Verbose logging and reporting.
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
}