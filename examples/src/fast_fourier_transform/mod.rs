// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    rescue, Example,
};
use crate::{ExampleOptions, utils::{print_trace, fast_fourier_transform::simple_iterative_fft}};
use log::debug;
use rand_utils::rand_array;
use std::time::Instant;
use winterfell::{
    math::{fields::f128::BaseElement, get_power_series, log2, FieldElement, StarkField, fft},
    ProofOptions, Prover, StarkProof, Trace, TraceTable, VerifierError,
};


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
        options.to_proof_options(28, 64),
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
// FIXME: Need to add constraints to check for input and output values.
pub struct FFTExample {
    options: ProofOptions,
    omega: BaseElement,
    num_fft_inputs: usize,
    fft_inputs: Vec<BaseElement>,
    result: Vec<BaseElement>,
}

impl FFTExample {
    pub fn new(num_fft_inputs: usize, options: ProofOptions) -> Self {
        assert!(
            num_fft_inputs.is_power_of_two(),
            "number of signatures must be a power of 2"
        );
        assert!(num_fft_inputs >= 4, "number of inputs must be at least 4");


        let mut fft_inputs = vec![BaseElement::ZERO; num_fft_inputs as usize];
        for internal_seed in fft_inputs.iter_mut() {
            *internal_seed = rand_array::<_, 1>()[0];
        }

        // compute the fft output using an external implementation of iterative FFT
        let now = Instant::now();
        // generate appropriately sized omega 
        let omega = BaseElement::get_root_of_unity(log2(num_fft_inputs));
        let result = simple_iterative_fft(fft_inputs.clone(), omega);

        debug!(
            "Generated {} coefficients in {} ms",
            num_fft_inputs,
            now.elapsed().as_millis()
        );

        

        FFTExample {
            options,
            omega,
            num_fft_inputs,
            fft_inputs,
            result,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl Example for FFTExample {
    fn prove(&self) -> StarkProof {
        // generate the execution trace
        debug!(
            "Generating proof for computing an FFT on {} inputs\n\
            ---------------------",
            self.num_fft_inputs,
        );

        // create a prover
        let prover =
            FFTProver::new(self.options.clone(), self.num_fft_inputs);
        let now = Instant::now();
        let trace = prover.build_trace(self.omega, &self.fft_inputs, &self.result);
        let trace_length = trace.length();
        debug!(
            "Generated execution trace of {} registers and 2^{} steps in {} ms",
            trace.width(),
            log2(trace_length),
            now.elapsed().as_millis()
        );
        // self.output_evals = prover.get_pub_inputs(&trace).output_evals;
        // generate the proof
        print_trace(&trace, 1, 0, 0..trace.width());
        prover.prove(trace).unwrap()
    }

    fn verify(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let pub_inputs = PublicInputs {
            result: self.result.clone(),
            num_inputs: self.num_fft_inputs,
            fft_inputs: self.fft_inputs.clone(),
        };
        
        winterfell::verify::<FFTAir>(proof, pub_inputs)
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let mut wrong_inputs = self.fft_inputs.clone();
        wrong_inputs[0] -= BaseElement::ONE;
        let pub_inputs = PublicInputs {
            result: self.result.clone(),
            num_inputs: self.num_fft_inputs,
            fft_inputs: self.fft_inputs.clone(),
        };
        winterfell::verify::<FFTAir>(proof, pub_inputs)
    }
}
