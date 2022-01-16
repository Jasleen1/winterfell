// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;

use math::fields::f128::BaseElement;

use models::arith_parser::R1CSArithReaderParser;

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

    let mut arith_file_parser = R1CSArithReaderParser::<BaseElement>::new().unwrap();
    arith_file_parser.parse_file(input_file, verbose);

    // let r1cs = arith_file_parser.r1cs_instance;
    // r1cs.debug_print();
}