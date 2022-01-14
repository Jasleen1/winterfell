use math::StarkField;
use regex::Regex;
use lazy_static::lazy_static;
use sscanf::scanf;

use crate::errors::*;
use crate::{index::*, r1cs::*};



#[derive(Clone, Debug)]
pub struct ArithParser {

}

pub trait ArithHandler {
    fn new() -> Self;
    fn process_line(&mut self, line: String);
    fn process_add(&mut self, tokens: &[&str]);
}

impl ArithParser {

    fn hdl_total(&mut self, wire_id: u32) {
        println!("GOT: total {}", wire_id);
    }
    fn hdl_input(&mut self, wire_id: u32) {
        println!("GOT: input {}", wire_id);
    }
    fn hdl_nizkinput(&mut self, wire_id: u32) {
        println!("GOT: nizkinput {}", wire_id);
    }
    fn hdl_output(&mut self, wire_id: u32) {
        println!("GOT: outinput {}", wire_id);
    }

    fn hdl_extended(&mut self, raw_cmd: String, in_args: String, out_args: String) {

        let in_vals = self.parse_index_vector(&in_args);
        let out_vals = self.parse_index_vector(&out_args);

        println!("EXTENDED: {:?} {:?}", in_vals, out_vals);

        match raw_cmd.as_str() {
            "add" => self.handle_add(in_vals, out_vals),
            "mul" => self.handle_mul(1, in_vals, out_vals),
            // "xor" => self.handle_xor(1, in_vals, out_vals),
            // "or" => self.handle_or(1, in_vals, out_vals),

            "const-mul" => self.handle_mul(100, in_vals, out_vals),
            "const-mul-neg" => self.handle_mul(-100, in_vals, out_vals),
            _ => println!("NOT HANDLED: {}", raw_cmd),
        }
    }

    fn proc_line(&mut self, line: String) {
        if line.starts_with("#") { return }

        // Remove comments and trim end-whitespace.
        let mut parts = line.split("#");
        let mut buf = parts.next().unwrap();
        buf = buf.trim();

        match scanf!(buf, "total {}", u32) { Some(wire_id) => { self.hdl_total(wire_id); return }, None => {}, }
        match scanf!(buf, "input {}", u32) { Some(wire_id) => { self.hdl_input(wire_id); return }, None => {}, }
        match scanf!(buf, "nizkinput {}", u32) { Some(wire_id) => { self.hdl_nizkinput(wire_id); return }, None => {}, }
        match scanf!(buf, "output {}", u32) { Some(wire_id) => { self.hdl_output(wire_id); return }, None => {}, }

        match scanf!(buf, "{} in {} <{}> out {} <{}>", String, u32, String, u32, String) {
            Some((raw_cmd, in_arity, in_args, out_arity, out_args)) => { self.hdl_extended(raw_cmd, in_args, out_args); return },
            None => {},
        }

        println!("NOT PROCESSED: {}", line);
    }

    fn initial_process(&mut self, line: String) -> (String, String, String) {
        // Remove comments and trim end-whitespace.
        let mut parts = line.split("#");
        let mut buf = parts.next().unwrap();
        buf = buf.trim();

        // Extract command and string with args.
        let mut parts = buf.splitn(2, ' ');
        let mut cmd = parts.next().unwrap();
        let mut argstr = "";
        match parts.next() { Some(x) => argstr = x, None => {}, };
        println!("[{}] {}", cmd, argstr);

        // Extract inlined command argument.
        let mut vparts: Vec<&str> = cmd.rsplitn(2, '-').collect();
        vparts.reverse();
        let mut cmd = vparts[0];
        let mut inline_arg = "";
        if vparts.len() > 1 {
            inline_arg = vparts[1];
        }
        println!("[{}] [{}] {}", cmd, inline_arg, argstr);

        println!("line: {}===", buf);

        (cmd.to_string(), inline_arg.to_string(), argstr.to_string())
    }

    fn parse_index_vector(&mut self, value: &str) -> Vec<u32> {
        // "<1 2 3>" or "<9>"
        lazy_static! {
            static ref LIST_RE: Regex = Regex::new(r"\d+").unwrap();
        }
        println!("    parse INDEX: {}", value);
        let mut vals: Vec<u32> = Vec::new();
        for num_cap in LIST_RE.captures_iter(&value) {
            println!("    capture idx: {}", &num_cap[0]);
            vals.push(num_cap[0].parse::<u32>().unwrap());
        }
        return vals;
    }

    fn parse_inout_args(&mut self, argstr: String) -> (Vec<u32>, Vec<u32>) {
        // "in 3 <1 2 3> out 2 <10 11>"
        lazy_static! {
            static ref BRAKET_RE: Regex = Regex::new(r"<(\d+(?:[ ]+\d+)*)>").unwrap();
        }
        let mut vals: Vec<String> = Vec::new();

        println!("CAPTURING {}", argstr);
        for cap in BRAKET_RE.captures_iter(&argstr) {
            println!(" capture: {}", &cap[0]);
            vals.push(cap[0].to_string());
        }
        (self.parse_index_vector(&vals[0]), self.parse_index_vector(&vals[1]))
    }

    fn handle_add(&mut self, in_args: Vec<u32>, out_args: Vec<u32>) {}
    fn handle_mul(&mut self, coeff: i32, in_args: Vec<u32>, out_args: Vec<u32>) {}
    fn handle_total(&mut self, total: u32) {}
    fn handle_input(&mut self, total: u32) {}
    fn handle_output(&mut self, total: u32) {}

    fn dispatch(&mut self, cmd: String, inline_arg: String, argstr: String) {
        let mut in_args: Vec<u32> = [].to_vec();
        let mut out_args: Vec<u32> = [].to_vec();
        let mut arg_value: u32 = 0;
        let mut inline_value: i32 = match inline_arg.as_str() { "" => 0, _ => inline_arg.parse().unwrap() };

        // Initialize values.
        match cmd.as_str() {
            "total" | "input" | "output" => {
                arg_value = argstr.parse().unwrap();
                in_args = [arg_value].to_vec();
                out_args = [].to_vec();
            },
            "add" | "mul" | "const-mul" | "const-mul-neg" => {
                arg_value = 0;
                let (i, o) = self.parse_inout_args(argstr);
                in_args = i;
                out_args = o;
            },
            _ => println!("DISPATCH UNRECOGNIZED COMMAND: {}", cmd),
        }

        println!("DISPATCH: {}; {}; {:?}; {:?}", inline_value, arg_value, in_args, out_args);

        match cmd.as_str() {
            "add" => self.handle_add(in_args, out_args),
            "mul" => self.handle_mul(1, in_args, out_args),
            "const-mul" => self.handle_mul(inline_value, in_args, out_args),
            "const-mul-neg" => self.handle_mul(-inline_value, in_args, out_args),
            "total" => self.handle_total(arg_value),
            "input" => self.handle_input(arg_value),
            "output" => self.handle_output(arg_value),
            _ => println!("UNRECOGNIZED: {}", cmd),
        }
    }

}

impl ArithHandler for ArithParser {

    fn new() -> Self {
        ArithParser {}
    }

    fn process_line(&mut self, line: String) {
        // println!("INGEST: {}", line);
        // if line.starts_with("#") { return };
        // self.proc_line(line);
        // return;

        let (cmd, inline_arg, argstr) = self.initial_process(line.clone());
        self.dispatch(cmd, inline_arg, argstr);
        return;

        let toks = line.split_whitespace();
        let tokens: Vec<&str> = toks.collect();

        if tokens.len() == 0 {
            return;
        }

        let cmd = tokens[0];

        println!("  COMMAND: {}", cmd);

        match cmd {
            "total" => {
                println!("  NOTIMPL TOTAL {}", cmd);
            },
            "input" => {
                println!("  NOTIMPL INPUT {}", cmd);
            },
            "add" => {
                println!("  ADD {}", cmd);
                self.process_add(&tokens[1..]);
            },
            _other => {
                println!("  NOTIMPL OTHER {}", cmd);
            },
        }

        // for tok in toks {
        //     match tok {
        //         "total" => {
        //             println!("  TOTAL {}", tok);
        //         },
        //         "input" => {
        //             println!("  INPUT {}", tok);
        //         },
        //         _other => {
        //             println!("  OTHER {}", tok);
        //         },
        //     }
        // }
    }



    fn process_add(&mut self, tokens: &[&str]) {
        for tok in tokens {
            println!("    {}", tok);
        }
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
