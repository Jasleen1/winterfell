use math::StarkField;

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

impl ArithHandler for ArithParser {

    fn new() -> Self {
        ArithParser {}
    }

    fn process_line(&mut self, line: String) {
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
