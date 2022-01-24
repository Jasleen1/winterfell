// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use structopt::StructOpt;

use crypto::hashers::Rp64_256;
use crypto::ElementHasher;
use fractal_proofs::log2;
use math::fields::f64::BaseElement;
use math::FieldElement;
use math::StarkField;

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
    orchestrate_r1cs_example::<BaseElement, BaseElement, Rp64_256, 16>(
        &options.arith_file,
        &options.wires_file,
        options.verbose);
}

pub(crate) fn orchestrate_r1cs_example<
    B: StarkField,
    E: FieldElement<BaseField = B>,
    H: ElementHasher + ElementHasher<BaseField = B>,
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
    let _wires = wires_parser.wires;

    // 1. Index this R1CS
    let index_params = IndexParams {
        num_input_variables: r1cs.num_cols().next_power_of_two(),
        num_constraints: r1cs.num_rows().next_power_of_two(),
        num_non_zero: r1cs.max_num_nonzero().next_power_of_two(),
    };

    let index_domains = build_index_domains::<B>(index_params.clone());
    let indexed_a = index_matrix::<B>(&r1cs.A, &index_domains);
    let indexed_b = index_matrix::<B>(&r1cs.B, &index_domains);
    let indexed_c = index_matrix::<B>(&r1cs.C, &index_domains);
    // This is the index i.e. the pre-processed data for this r1cs
    let index = Index::new(index_params, indexed_a, indexed_b, indexed_c);

    let (_prover_key, _verifier_key) = generate_prover_and_verifier_keys::<H, B, N>(index).unwrap();

    // TODO: create FractalProver
    let degree_fs = r1cs.clone().num_cols();
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
    //     pub_inputs_bytes
    // );
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