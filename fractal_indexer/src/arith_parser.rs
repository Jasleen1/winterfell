use std::convert::TryInto;
use std::marker::PhantomData;

use math::StarkField;
use regex::Regex;
use lazy_static::lazy_static;
use sscanf::scanf;

use crate::errors::*;
use crate::r1cs::*;

#[derive(Clone, Debug)]
pub struct ArithParser<E: StarkField> {
    pub verbose: bool,
    r1cs_instance: R1CS<E>,
}

pub trait LineProcessor {
    fn process_line(&mut self, line: String);
}

impl<E: StarkField> ArithParser<E> {

    pub fn new() -> Result<Self, R1CSError> {
        Ok(ArithParser {
            verbose: false,
            r1cs_instance: create_empty_r1cs()?
        })
    }

    pub fn return_r1cs(&self) -> R1CS<E> {
        self.r1cs_instance.clone()
    }

    // Handlers.
    fn handle_total(&mut self, total: u32) {
        if self.verbose { println!("TOTAL: {}", total) };
        self.r1cs_instance.set_cols(total.try_into().unwrap());
    }

    fn handle_input(&mut self, wire_id: u32) {
        println!("NOTIMPL INPUT: {}", wire_id);
    }

    fn handle_nizkinput(&mut self, wire_id: u32) {
        println!("NOTIMPL NIZKINPUT: {}", wire_id);
    }

    fn handle_output(&mut self, wire_id: u32) {
        println!("NOTIMPL OUTPUT: {}", wire_id);
    }

    fn handle_add(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        if self.verbose { println!("ADD: {:?} {:?}", in_args, out_args) };
        let mut new_row_a = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_b = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_c = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let a_pos_1: usize = in_args[0].try_into().unwrap();
        let a_pos_2: usize = in_args[1].try_into().unwrap();
        let c_pos: usize = out_args[0].try_into().unwrap();

        new_row_a[a_pos_1] = E::ONE;
        new_row_a[a_pos_2] = E::ONE;
        new_row_b[0] = E::ONE;
        new_row_c[c_pos] = E::ONE;

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_const_add(&mut self, coeff: i32, in_args: Vec<u32>, out_args: Vec<u32>) {
        if self.verbose { println!("CONST ADD: {} {:?} {:?}", coeff, in_args, out_args) };
        let mut new_row_a = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_b = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_c = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let a_pos_1: usize = coeff.try_into().unwrap();
        let a_val_2: u64 = in_args[0].try_into().unwrap();
        let c_pos: usize = out_args[0].try_into().unwrap();

        new_row_a[a_pos_1] = E::ONE;
        new_row_a[0] = E::from(a_val_2);
        new_row_b[0] = E::ONE;
        new_row_c[c_pos] = E::ONE;

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_mul(&mut self, coeff: i32, in_args: Vec<u32>, out_args: Vec<u32>) {
        if self.verbose { println!("MUL: {} {:?} {:?}", coeff, in_args, out_args) };
        
        let mut new_row_a = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_b = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_c = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let a_pos: usize = in_args[0].try_into().unwrap();
        
        let c_pos: usize = out_args[0].try_into().unwrap();
        let coeff_u64: u64 = coeff.try_into().unwrap();
        new_row_a[a_pos] = E::from(coeff_u64);
        if in_args.len() > 1 {
            let b_pos: usize = in_args[1].try_into().unwrap();
            new_row_b[b_pos] = E::ONE; 
        }
        else {
            new_row_b[0] = E::ONE;
        }
        new_row_c[c_pos] = E::ONE;

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_xor(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        if self.verbose { println!("NOTIMPL XOR: {:?} {:?}", in_args, out_args) };
        let mut new_row_a = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_b = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_c = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        // a + b - 2*ab = a XOR b so, 2a*b = a + b - a XOR b. 
        let a_pos: usize = in_args[0].try_into().unwrap();
        let b_pos: usize = in_args[1].try_into().unwrap();
        let c_pos: usize = out_args[0].try_into().unwrap();

        new_row_a[a_pos] = E::from(2u64);
        new_row_b[b_pos] = E::ONE;
        new_row_c[a_pos] = E::ONE;
        new_row_c[b_pos] = E::ONE;
        new_row_c[c_pos] = E::ONE.neg();

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_or(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        if self.verbose { println!("NOTIMPL OR: {:?} {:?}", in_args, out_args) } ;
        let mut new_row_a = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_b = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        let mut new_row_c = vec![E::ZERO; self.r1cs_instance.get_num_cols()];
        // a + b - ab = a OR b so, a*b = a + b - a OR b. 
        let a_pos: usize = in_args[0].try_into().unwrap();
        let b_pos: usize = in_args[1].try_into().unwrap();
        let c_pos: usize = out_args[0].try_into().unwrap();

        new_row_a[a_pos] = E::ONE;
        new_row_b[b_pos] = E::ONE;
        new_row_c[a_pos] = E::ONE;
        new_row_c[b_pos] = E::ONE;
        new_row_c[c_pos] = E::ONE.neg();

        self.r1cs_instance.add_rows(new_row_a, new_row_b, new_row_c);
    }

    fn handle_nonzero(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        println!("NOTIMPL NONZERO: {:?} {:?}", in_args, out_args);
    }

    // An extended command.
    fn handle_extended(&mut self, raw_cmd: String, in_args: String, out_args: String) {
        let element_one = 1;  // for now, i32, but will want to use field elements.

        let in_vals = self.parse_index_vector(&in_args);
        let out_vals = self.parse_index_vector(&out_args);

        // Commands with implicit coefficients (part of the command name itself): MULTIPLICATION
        match scanf!(raw_cmd, "const-mul-{x}", i32) { Some(coeff) => { self.handle_mul(coeff, in_vals, out_vals); return }, None => {}, }
        match scanf!(raw_cmd, "const-mul-neg-{x}", i32) { Some(coeff) => { self.handle_mul(-coeff, in_vals, out_vals); return }, None => {}, }

        // Commands with implicit coefficients (part of the command name itself): ADDITION
        match scanf!(raw_cmd, "const-add-{x}", i32) { Some(coeff) => { self.handle_const_add(coeff, in_vals, out_vals); return }, None => {}, }
        match scanf!(raw_cmd, "const-add-neg-{x}", i32) { Some(coeff) => { self.handle_const_add(-coeff, in_vals, out_vals); return }, None => {}, }

        // Commands with lots of inputs and outputs.
        match raw_cmd.as_str() {
            "add" => self.handle_add(in_vals, out_vals),
            "mul" => self.handle_mul(element_one, in_vals, out_vals),
            "xor" => self.handle_xor(in_vals, out_vals),
            "or" => self.handle_or(in_vals, out_vals),
            "zerop" => self.handle_nonzero(in_vals, out_vals),
            _ => println!("NOT HANDLED: {}", raw_cmd),
        }
    }

    // Parse the arith bra-ket format for vectors of indices (eg "<1 2 3>" or "<9>")
    fn parse_index_vector(&mut self, value: &str) -> Vec<u32> {
        lazy_static! {
            static ref LIST_RE: Regex = Regex::new(r"\d+").unwrap();
        }
        let mut vals: Vec<u32> = Vec::new();
        for num_cap in LIST_RE.captures_iter(&value) {
            vals.push(num_cap[0].parse::<u32>().unwrap());
        }
        return vals;
    }
}

impl<E: StarkField> LineProcessor for ArithParser<E> {
    fn process_line(&mut self, line: String) {
        if self.verbose {
            println!("{}", line);
        }
        if line.starts_with("#") { return }

        // Remove comments and trim end-whitespace.
        let mut parts = line.split("#");
        let mut buf = parts.next().unwrap();
        buf = buf.trim();

        // Arity 1 commands:
        match scanf!(buf, "total {}", u32) { Some(total) => { self.handle_total(total); return }, None => {}, }
        match scanf!(buf, "input {}", u32) { Some(wire_id) => { self.handle_input(wire_id); return }, None => {}, }
        match scanf!(buf, "nizkinput {}", u32) { Some(wire_id) => { self.handle_nizkinput(wire_id); return }, None => {}, }
        match scanf!(buf, "output {}", u32) { Some(wire_id) => { self.handle_output(wire_id); return }, None => {}, }

        // Extended commands, including with implicit inputs (coefficients):
        match scanf!(buf, "{} in {} <{}> out {} <{}>", String, u32, String, u32, String) {
            Some((raw_cmd, _in_arity, in_args, _out_arity, out_args)) => { self.handle_extended(raw_cmd, in_args, out_args); return },
            None => {},
        }

        println!("FAILED: {}", line);
    }
}