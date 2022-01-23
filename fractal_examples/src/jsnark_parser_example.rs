// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;

use math::fields::f128::BaseElement;

use models::jsnark_arith_parser::R1CSArithReaderParser;
use models::jsnark_wire_parser::JsnarkWireReaderParser;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut arith_file = "./src/sample.arith";
    let mut wire_file = "./src/sample.in";
    if args.len() > 1 {
        arith_file = &args[1];
    }
    if args.len() > 2 {
        wire_file = &args[2];
    }

    let verbose = true;
    if verbose {
        println!("Parse files {} {}", arith_file, wire_file);
    }

    let mut arith_file_parser = R1CSArithReaderParser::<BaseElement>::new().unwrap();
    arith_file_parser.parse_arith_file(&arith_file, verbose);
    let r1cs_instance = arith_file_parser.r1cs_instance;

    let mut wire_file_parser = JsnarkWireReaderParser::<BaseElement>::new().unwrap();
    wire_file_parser.parse_wire_file(&wire_file, verbose);
    let wires = wire_file_parser.wires;

    if verbose {
        r1cs_instance.debug_print_bits_horizontal();
        r1cs_instance.debug_print_symbolic();
        println!("{:?}", wires);
    }
}
