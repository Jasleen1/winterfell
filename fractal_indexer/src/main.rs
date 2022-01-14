// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use fractal_indexer::arith_parser::{ArithParser, LineProcessor};
use math::fields::f128::BaseElement;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut input_file = "sample.arith";
    if args.len() > 1 {
        input_file = &args[1];
    }
    let verbose = true;
    println!("Parse file {}", input_file);

    // let mut arith_parser: arith_parser::ArithParser = arith_parser::LineProcessor::new();
    let mut arith_parser = ArithParser::<BaseElement>::new().unwrap();
    arith_parser.verbose = verbose;

    if let Ok(lines) = read_lines(input_file) {
        for line in lines {
            match line {
                Ok(ip) => {
                    arith_parser.process_line(ip);
                },
                Err(e) => println!("{:?}", e),
            }
        }
    }

    // println!("{:?}", arith_parser.return_r1cs());
    if arith_parser.verbose {
        let mut r1cs = arith_parser.return_r1cs();
        r1cs.debug_print_bits_horizontal();
        r1cs.debug_print_symbolic();
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = match File::open(filename) {
        Err(why) => panic!("Cannot open file: {}", why),
        Ok(file) => file,
    };
    Ok(io::BufReader::new(file).lines())
}