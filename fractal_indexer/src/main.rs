// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

// use log::debug;
// use std::io::Write;
//use std::time::Instant;
//use structopt::StructOpt;
//use winterfell::StarkProof;

use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

//use crate::{index::*, r1cs::*};

// use std::vec;

use fractal_indexer::arith_parser;
use fractal_indexer::arith_parser::ArithHandler;


// use errors::R1CSError;
// use index::*;

// use math::{
//     fields::f128::{self, BaseElement},
// };

// type SmallFieldElement17 = math::fields::smallprimefield::BaseElement<17, 3, 4>;

fn main() {
    println!("Fractal Indexer");
    let args: Vec<String> = env::args().collect();

    let mut input_file = "sample.arith";
    if args.len() > 1 {
        input_file = &args[1];
    }
    println!("Parse file {}", input_file);

    // let mata = make_all_zeros_matrix_f17("AAA", 10, 10).unwrap();
    // let matb = make_all_zeros_matrix_f17("BBB", 10, 10).unwrap();
    // let matc = make_all_zeros_matrix_f17("CCC", 10, 10).unwrap();

    // let mut arith_parser = arith_parser::ArithParser {
    //     A: mata,
    //     B: matb,
    //     C: matc
    // };

    let mut arith_parser: arith_parser::ArithParser = arith_parser::ArithHandler::new();
    // let mut arith_parser = arith_parser::ArithParser {};

    if let Ok(lines) = read_lines(input_file) {
        for line in lines {
            if let Ok(ip) = line {
                println!("{}", ip);
                arith_parser.process_line(ip);
            }
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}



// fn make_all_zeros_matrix_f17(
//     matrix_name: &str,
//     rows: usize,
//     cols: usize,
// ) -> Result<Matrix<SmallFieldElement17>, R1CSError>  {
//     let mut mat = Vec::new();
//     let new_row = vec![SmallFieldElement17::new(0); cols];
//     for _i in 0..rows {
//         mat.push(new_row.clone());
//     }
//     Matrix::new(matrix_name, mat)
// }