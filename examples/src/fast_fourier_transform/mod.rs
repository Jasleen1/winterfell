// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    rescue, Example,
};
use crate::ExampleOptions;
use log::debug;
use std::time::Instant;
use winterfell::{
    math::{fields::f128::BaseElement, get_power_series, log2, FieldElement, StarkField},
    ProofOptions, Prover, StarkProof, Trace, TraceTable, VerifierError,
};

use utils::Randomizable;

mod air;
use air::{FFTAir, PublicInputs};

mod prover;
use prover::FFTProver;

// CONSTANTS
// ================================================================================================

// const TRACE_WIDTH: usize = 22;
// const SIG_CYCLE_LENGTH: usize = 128 * CYCLE_LENGTH; // 1024 steps

// Field FFT EXAMPLE
// ================================================================================================
pub fn get_example(options: ExampleOptions, degree: usize) -> Box<dyn Example> {
    Box::new(FFTExample::new(
        degree,
        options.to_proof_options(28, 8),
    ))
}

/*

    """
    Given coefficients A of polynomial this method does FFT and returns
    the evaluation of the polynomial at [omega^0, omega^(n-1)]
    If the polynomial is a0*x^0 + a1*x^1 + ... + an*x^n then the coefficients
    list is of the form coefficients := [a0, a1, ... , an].
    """
*/

pub struct FFTExample {
    options: ProofOptions,
    coefficients: Vec<BaseElement>,
    omega: BaseElement,
    degree: usize,
}

impl FFTExample {
    pub fn new(degree: usize, options: ProofOptions) -> Self {
        assert!(
            degree.is_power_of_two(),
            "number of signatures must be a power of 2"
        );
        assert!(
            degree <= BaseElement::TWO_ADICITY.try_into().unwrap(), 
            "Provided degree is too large"
        );
        // generate appropriately sized omega 
        let omega = BaseElement::get_root_of_unity(log2(degree)); 
        // generate appropriately sized coefficients
        let mut coefficients = Vec::with_capacity(degree);
        let now = Instant::now();
        for i in 0..degree {
            coefficients.push(BaseElement::from_random_bytes(&[i as u8; 32]).unwrap());
        }
        debug!(
            "Generated {} coefficients in {} ms",
            degree,
            now.elapsed().as_millis()
        );

        FFTExample {
            options,
            coefficients,
            omega,
            degree,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for FFTExample {
    fn prove(&self) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for verifying degree {} FFT \n\
            ---------------------",
            self.degree,
        );

        // create a prover
        // let prover =
            // FFTProver::new(&self.pub_keys, &self.messages, self.options.clone());

        // let now = Instant::now();
        // let trace = prover.build_trace(&self.messages, &self.signatures);
        // let trace_length = trace.length();
        // debug!(
        //     "Generated execution trace of {} registers and 2^{} steps in {} ms",
        //     trace.width(),
        //     log2(trace_length),
        //     now.elapsed().as_millis()
        // );

        // // generate the proof
        // prover.prove(trace).unwrap()
        unimplemented!()
    }

    fn verify(&self, proof: StarkProof) -> Result<(), VerifierError> {
        // let pub_inputs = PublicInputs {
        //     pub_keys: self.pub_keys.clone(),
        //     messages: self.messages.clone(),
        // };
        // winterfell::verify::<FFTAir>(proof, pub_inputs)
        unimplemented!()
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        // let mut pub_keys = self.pub_keys.clone();
        // pub_keys.swap(0, 1);
        // let pub_inputs = PublicInputs {
        //     pub_keys,
        //     messages: self.messages.clone(),
        // };
        // winterfell::verify::<FFTAir>(proof, pub_inputs)
        unimplemented!()
    }
}
