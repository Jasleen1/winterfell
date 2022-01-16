use lazy_static::lazy_static;
use math::StarkField;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use sscanf::scanf;

use crate::errors::*;
use crate::r1cs::*;

#[derive(Clone, Debug)]
pub struct R1CSArithParser<E: StarkField> {
    pub verbose: bool,
    r1cs_instance: R1CS<E>,
}

pub trait LineProcessor {
    fn process_line(&mut self, line: String);
}

impl<E: StarkField> R1CSArithParser<E> {
    pub fn new() -> Result<Self, R1CSError> {
        Ok(R1CSArithParser {
            verbose: false,
            r1cs_instance: create_empty_r1cs()?,
        })
    }

    pub fn return_r1cs(&self) -> R1CS<E> {
        self.r1cs_instance.clone()
    }

    // Handlers.
    fn handle_total(&mut self, total: usize) {
        if self.verbose {
            println!("TOTAL: {}", total)
        };
        self.r1cs_instance.set_cols(total);
    }

    fn handle_input(&mut self, wire_id: usize) {
        println!("NOTIMPL INPUT: {}", wire_id);
    }

    fn handle_nizkinput(&mut self, wire_id: usize) {
        println!("NOTIMPL NIZKINPUT: {}", wire_id);
    }

    fn handle_output(&mut self, wire_id: usize) {
        println!("NOTIMPL OUTPUT: {}", wire_id);
    }

    fn handle_add(&mut self, coeff: E, in_args: Vec<usize>, out_args: Vec<usize>) {
        if self.verbose {
            println!("CONST ADD: {} {:?} {:?}", coeff, in_args, out_args)
        };

        let numcols = self.r1cs_instance.get_num_cols();
        let mut new_row_a = vec![E::ZERO; numcols];
        let mut new_row_b = vec![E::ZERO; numcols];
        let mut new_row_c = vec![E::ZERO; numcols];

        let c_pos = out_args[0];

        for a_pos in in_args {
            new_row_a[a_pos] = E::ONE;
        }
        new_row_a[0] = coeff;
        new_row_b[0] = E::ONE;
        new_row_c[c_pos] = E::ONE;

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_mul(&mut self, coeff: E, in_args: Vec<usize>, out_args: Vec<usize>) {
        if self.verbose {
            println!("MUL: {} {:?} {:?}", coeff, in_args, out_args)
        };

        let numcols = self.r1cs_instance.get_num_cols();
        let mut new_row_a = vec![E::ZERO; numcols];
        let mut new_row_b = vec![E::ZERO; numcols];
        let mut new_row_c = vec![E::ZERO; numcols];

        let a_pos = in_args[0];
        let c_pos = out_args[0];

        new_row_a[a_pos] = E::from(coeff);
        if in_args.len() > 1 {
            let b_pos = in_args[1];
            new_row_b[b_pos] = E::ONE;
        } else {
            new_row_b[0] = E::ONE;
        }
        new_row_c[c_pos] = E::ONE;

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_xor(&mut self, in_args: Vec<usize>, out_args: Vec<usize>) {
        if self.verbose {
            println!("XOR: {:?} {:?}", in_args, out_args)
        };

        let numcols = self.r1cs_instance.get_num_cols();
        let mut new_row_a = vec![E::ZERO; numcols];
        let mut new_row_b = vec![E::ZERO; numcols];
        let mut new_row_c = vec![E::ZERO; numcols];

        let a_pos = in_args[0];
        let b_pos = in_args[1];
        let c_pos = out_args[0];

        // a + b - 2*ab = a XOR b so, 2a*b = a + b - a XOR b.
        new_row_a[a_pos] = E::from(2u64);
        new_row_b[b_pos] = E::ONE;
        new_row_c[a_pos] = E::ONE;
        new_row_c[b_pos] = E::ONE;
        new_row_c[c_pos] = E::ONE.neg();

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_or(&mut self, in_args: Vec<usize>, out_args: Vec<usize>) {
        if self.verbose {
            println!("OR: {:?} {:?}", in_args, out_args)
        };

        let numcols = self.r1cs_instance.get_num_cols();
        let mut new_row_a = vec![E::ZERO; numcols];
        let mut new_row_b = vec![E::ZERO; numcols];
        let mut new_row_c = vec![E::ZERO; numcols];

        let a_pos = in_args[0];
        let b_pos = in_args[1];
        let c_pos = out_args[0];

        // a + b - ab = a OR b so, a*b = a + b - a OR b.
        new_row_a[a_pos] = E::ONE;
        new_row_b[b_pos] = E::ONE;
        new_row_c[a_pos] = E::ONE;
        new_row_c[b_pos] = E::ONE;
        new_row_c[c_pos] = E::ONE.neg();

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_nonzero(&mut self, in_args: Vec<usize>, out_args: Vec<usize>) {
        println!("NOTIMPL NONZERO: {:?} {:?}", in_args, out_args);
    }

    // An extended command.
    fn handle_extended(&mut self, raw_cmd: String, in_args: String, out_args: String) {
        let in_vals = self.parse_index_vector(&in_args);
        let out_vals = self.parse_index_vector(&out_args);

        // Commands with implicit coefficients (part of the command name itself): MULTIPLICATION
        match scanf!(raw_cmd, "const-mul-{x}", u64) {
            Some(coeff) => {
                self.handle_mul(E::from(coeff), in_vals, out_vals);
                return;
            }
            None => {}
        }
        match scanf!(raw_cmd, "const-mul-neg-{x}", u64) {
            Some(coeff) => {
                self.handle_mul(E::from(coeff).neg(), in_vals, out_vals);
                return;
            }
            None => {}
        }

        // Commands with implicit coefficients (part of the command name itself): ADDITION
        match scanf!(raw_cmd, "const-add-{x}", u64) {
            Some(coeff) => {
                self.handle_add(E::from(coeff), in_vals, out_vals);
                return;
            }
            None => {}
        }
        match scanf!(raw_cmd, "const-add-neg-{x}", u64) {
            Some(coeff) => {
                self.handle_add(E::from(coeff).neg(), in_vals, out_vals);
                return;
            }
            None => {}
        }

        // Commands with lots of inputs and outputs.
        match raw_cmd.as_str() {
            "add" => self.handle_add(E::ZERO, in_vals, out_vals),
            "mul" => self.handle_mul(E::ONE, in_vals, out_vals),
            "xor" => self.handle_xor(in_vals, out_vals),
            "or" => self.handle_or(in_vals, out_vals),
            "zerop" => self.handle_nonzero(in_vals, out_vals),
            _ => println!("NOT HANDLED: {}", raw_cmd),
        }
    }

    // Parse the arith bra-ket format for vectors of indices (eg "<1 2 3>" or "<9>")
    fn parse_index_vector(&mut self, value: &str) -> Vec<usize> {
        lazy_static! {
            static ref LIST_RE: Regex = Regex::new(r"\d+").unwrap();
        }
        let mut vals: Vec<usize> = Vec::new();
        for num_cap in LIST_RE.captures_iter(&value) {
            vals.push(num_cap[0].parse::<usize>().unwrap());
        }
        return vals;
    }
}

impl<E: StarkField> LineProcessor for R1CSArithParser<E> {
    fn process_line(&mut self, line: String) {
        if self.verbose {
            println!("{}", line);
        }
        if line.starts_with("#") {
            return;
        }

        // Remove comments and trim end-whitespace.
        let mut parts = line.split("#");
        let mut buf = parts.next().unwrap();
        buf = buf.trim();

        // Arity 1 commands:
        match scanf!(buf, "total {}", usize) {
            Some(total) => {
                self.handle_total(total);
                return;
            }
            None => {}
        }
        match scanf!(buf, "input {}", usize) {
            Some(wire_id) => {
                self.handle_input(wire_id);
                return;
            }
            None => {}
        }
        match scanf!(buf, "nizkinput {}", usize) {
            Some(wire_id) => {
                self.handle_nizkinput(wire_id);
                return;
            }
            None => {}
        }
        match scanf!(buf, "output {}", usize) {
            Some(wire_id) => {
                self.handle_output(wire_id);
                return;
            }
            None => {}
        }

        // Extended commands, including with implicit inputs (coefficients):
        match scanf!(
            buf,
            "{} in {} <{}> out {} <{}>",
            String,
            u32,
            String,
            u32,
            String
        ) {
            Some((raw_cmd, _in_arity, in_args, _out_arity, out_args)) => {
                self.handle_extended(raw_cmd, in_args, out_args);
                return;
            }
            None => {}
        }

        println!("FAILED: {}", line);
    }
}

pub struct R1CSArithReaderParser<E: StarkField> {
    pub r1cs_instance: R1CS<E>,
}

impl<E: StarkField> R1CSArithReaderParser<E> {

    pub fn new() -> Result<Self, R1CSError> {
        Ok(R1CSArithReaderParser {
            r1cs_instance: create_empty_r1cs()?,
        })
    }

    pub fn parse_file(&mut self, input_file: &str, verbose: bool) {
        if verbose {
            println!("Parse file {}", input_file);
        }

        // let mut arith_parser: arith_parser::ArithParser = arith_parser::LineProcessor::new();
        let mut arith_parser = R1CSArithParser::<E>::new().unwrap();
        arith_parser.verbose = verbose;

        if let Ok(lines) = self.read_lines(input_file) {
            for line in lines {
                match line {
                    Ok(ip) => {
                        arith_parser.process_line(ip);
                    }
                    Err(e) => println!("{:?}", e),
                }
            }
        }

        self.r1cs_instance = arith_parser.return_r1cs();
        // println!("{:?}", arith_parser.return_r1cs());
        if arith_parser.verbose {
            self.r1cs_instance.debug_print_bits_horizontal();
            self.r1cs_instance.debug_print_symbolic();
        }
    }

    fn read_lines<P>(&self, filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
        where P: AsRef<Path>,
    {
        let file = match File::open(filename) {
            Err(why) => panic!("Cannot open file: {}", why),
            Ok(file) => file,
        };
        Ok(io::BufReader::new(file).lines())
    }
}