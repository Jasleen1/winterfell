use math::StarkField;
use regex::Regex;
use lazy_static::lazy_static;
use sscanf::scanf;

use crate::errors::*;
use crate::{index::*, r1cs::*};



#[derive(Clone, Debug)]
pub struct ArithParser {

}

pub trait LineProcessor {
    fn new() -> Self;
    fn process_line(&mut self, line: String);
}

impl ArithParser {

    // Handlers.
    fn handle_total(&mut self, wire_id: u32) {
        println!("NOTIMPL TOTAL: {}", wire_id);
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
        println!("NOTIMPL ADD: {:?} {:?}", in_args, out_args);
    }
    fn handle_mul(&mut self, coeff: i32, in_args: Vec<u32>, out_args: Vec<u32>) {
        println!("NOTIMPL MUL: {} {:?} {:?}", coeff, in_args, out_args);
    }
    fn handle_xor(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        println!("NOTIMPL XOR: {:?} {:?}", in_args, out_args);
    }
    fn handle_or(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        println!("NOTIMPL OR: {:?} {:?}", in_args, out_args);
    }
    fn handle_nonzero(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {
        println!("NOTIMPL NONZERO: {:?} {:?}", in_args, out_args);
    }

    // An extended command.
    fn handle_extended(&mut self, raw_cmd: String, in_args: String, out_args: String) {
        let elementOne = 1;  // for now, i32, but will want to use field elements.

        let in_vals = self.parse_index_vector(&in_args);
        let out_vals = self.parse_index_vector(&out_args);

        // Commands with implicit coefficients (part of the command name itself).
        match scanf!(raw_cmd, "const-mul-{x}", i32) { Some(coeff) => { self.handle_mul(coeff, in_vals, out_vals); return }, None => {}, }
        match scanf!(raw_cmd, "const-mul-neg-{x}", i32) { Some(coeff) => { self.handle_mul(-coeff, in_vals, out_vals); return }, None => {}, }

        // Commands with lots of inputs and outputs.
        match raw_cmd.as_str() {
            "add" => self.handle_add(in_vals, out_vals),
            "mul" => self.handle_mul(elementOne, in_vals, out_vals),
            "xor" => self.handle_xor(in_vals, out_vals),
            "or" => self.handle_or(in_vals, out_vals),
            "zerop" => self.handle_nonzero(in_vals, out_vals),
            _ => println!("NOT HANDLED: {}", raw_cmd),
        }
    }

    fn ingest_line(&mut self, line: String) {
        if line.starts_with("#") { return }

        // Remove comments and trim end-whitespace.
        let mut parts = line.split("#");
        let mut buf = parts.next().unwrap();
        buf = buf.trim();

        // Arity 1 commands:
        match scanf!(buf, "total {}", u32) { Some(wire_id) => { self.handle_total(wire_id); return }, None => {}, }
        match scanf!(buf, "input {}", u32) { Some(wire_id) => { self.handle_input(wire_id); return }, None => {}, }
        match scanf!(buf, "nizkinput {}", u32) { Some(wire_id) => { self.handle_nizkinput(wire_id); return }, None => {}, }
        match scanf!(buf, "output {}", u32) { Some(wire_id) => { self.handle_output(wire_id); return }, None => {}, }

        // Extended commands, including with implicit inputs (coefficients):
        match scanf!(buf, "{} in {} <{}> out {} <{}>", String, u32, String, u32, String) {
            Some((raw_cmd, in_arity, in_args, out_arity, out_args)) => { self.handle_extended(raw_cmd, in_args, out_args); return },
            None => {},
        }

        println!("FAILED: {}", line);
    }

    fn parse_index_vector(&mut self, value: &str) -> Vec<u32> {
        // "<1 2 3>" or "<9>"
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

impl LineProcessor for ArithParser {

    fn new() -> Self {
        ArithParser {}
    }

    fn process_line(&mut self, line: String) {
        println!("INGEST: {}", line);
        self.ingest_line(line);
    }
}



// pub struct ArithParser<E: StarkField> {
//     pub A: Matrix<E>,
//     pub B: Matrix<E>,
//     pub C: Matrix<E>,
// }

// impl<E: StarkField> ArithParser<E> {
//     fn process_line() {

//     }
// }
