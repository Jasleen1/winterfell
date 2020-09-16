use crate::ConstraintDomain;
use math::polynom;
use serde::{Deserialize, Serialize};

// TRACE TABLE
// ================================================================================================
pub struct TraceTable(Vec<Vec<u128>>);

impl TraceTable {
    pub fn new(registers: Vec<Vec<u128>>) -> TraceTable {
        assert!(
            registers.len() > 0,
            "execution trace must consist of at least one register"
        );
        let trace_length = registers[0].len();
        assert!(
            trace_length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );
        for register in registers.iter() {
            assert!(
                register.len() == trace_length,
                "all register traces must have the same length"
            );
        }

        TraceTable(registers)
    }

    pub fn num_states(&self) -> usize {
        self.0[0].len()
    }

    pub fn num_registers(&self) -> usize {
        self.0.len()
    }

    pub fn copy_row(&self, idx: usize, destination: &mut [u128]) {
        for (i, register) in self.0.iter().enumerate() {
            destination[i] = register[idx];
        }
    }

    pub fn get(&self, register: usize, step: usize) -> u128 {
        self.0[register][step]
    }

    #[cfg(test)]
    pub fn get_register(&self, idx: usize) -> &[u128] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<u128>> {
        self.0
    }
}

// POLYNOMIAL TABLE
// ================================================================================================
pub struct PolyTable(Vec<Vec<u128>>);

impl PolyTable {
    pub fn new(polys: Vec<Vec<u128>>) -> PolyTable {
        assert!(
            polys.len() > 0,
            "polynomial table must contain at least one polynomial"
        );
        let poly_size = polys[0].len();
        assert!(
            poly_size.is_power_of_two(),
            "polynomial size must be a power of 2"
        );
        for poly in polys.iter() {
            assert!(
                poly.len() == poly_size,
                "all polynomials must have the same size"
            );
        }

        PolyTable(polys)
    }

    pub fn poly_size(&self) -> usize {
        self.0[0].len()
    }

    /// Evaluates all polynomials the the specified point `x`.
    pub fn evaluate_at(&self, x: u128) -> Vec<u128> {
        let mut result = Vec::with_capacity(self.num_polys());
        for poly in self.0.iter() {
            result.push(polynom::eval(&poly, x));
        }
        result
    }

    pub fn num_polys(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub fn get_poly(&self, idx: usize) -> &[u128] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<u128>> {
        self.0
    }
}

// CONSTRAINT EVALUATION TABLE
// ================================================================================================
pub struct ConstraintEvaluationTable(Vec<Vec<u128>>, Vec<ConstraintDomain>);

impl ConstraintEvaluationTable {
    pub fn new(
        transition: Vec<u128>,
        input: Vec<u128>,
        output: Vec<u128>,
        domains: Vec<ConstraintDomain>,
    ) -> Self {
        // TODO: verify lengths
        ConstraintEvaluationTable(vec![transition, input, output], domains)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn domains(&self) -> &[ConstraintDomain] {
        &self.1
    }

    pub fn transition_evaluations(&self) -> &[u128] {
        &self.0[0]
    }

    pub fn input_evaluations(&self) -> &[u128] {
        &self.0[1]
    }

    pub fn output_evaluations(&self) -> &[u128] {
        &self.0[2]
    }

    pub fn into_vec(self) -> Vec<Vec<u128>> {
        self.0
    }
}

// CONSTRAINT POLYNOMIAL
// ================================================================================================
pub struct ConstraintPoly(Vec<u128>);

impl ConstraintPoly {
    pub fn new(coefficients: Vec<u128>) -> Self {
        ConstraintPoly(coefficients)
    }

    pub fn degree(&self) -> usize {
        // TODO
        0
    }

    pub fn into_vec(self) -> Vec<u128> {
        self.0
    }
}

// FRI LAYER & FRI PROOF
// ================================================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriLayer {
    pub root: [u8; 32],
    pub values: Vec<[u128; 4]>,
    pub nodes: Vec<Vec<[u8; 32]>>,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriProof {
    pub layers: Vec<FriLayer>,
    pub rem_root: [u8; 32],
    pub rem_values: Vec<u128>,
}
