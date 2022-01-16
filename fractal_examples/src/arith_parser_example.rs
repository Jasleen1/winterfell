// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use math::StarkField;

use models::arith_parser::{LineProcessor, R1CSArithParser};
use models::r1cs::R1CS;

pub(crate) fn reading_arith<E: StarkField>(input_file: &str, _verbose: bool) -> R1CS<E> {
    // Use the below commented code if you want to read a
    // specific file, provided as a command-line arg
    // let args: Vec<String> = env::args().collect();
    // let current_path = env::current_dir().unwrap();
    // println!("in: {:?}", current_path);
    // let mut input_file = "./src/sample.arith";
    // if args.len() > 1 {
    //     input_file = &args[1];
    // }

    // let input_file = "./src/sample.arith";

    let verbose = true;
    println!("Parse file {}", input_file);

    // let mut arith_parser: arith_parser::ArithParser = arith_parser::LineProcessor::new();
    let mut arith_parser = R1CSArithParser::<E>::new().unwrap();
    arith_parser.verbose = verbose;

    if let Ok(lines) = read_lines(input_file) {
        for line in lines {
            match line {
                Ok(ip) => {
                    arith_parser.process_line(ip);
                }
                Err(e) => println!("{:?}", e),
            }
        }
    }
    let mut r1cs = arith_parser.return_r1cs();
    // println!("{:?}", arith_parser.return_r1cs());
    if arith_parser.verbose {
        r1cs.debug_print_bits_horizontal();
        r1cs.debug_print_symbolic();
    }
    r1cs
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = match File::open(filename) {
        Err(why) => panic!("Cannot open file: {}", why),
        Ok(file) => file,
    };
    Ok(io::BufReader::new(file).lines())
}
