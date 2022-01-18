use std::convert::TryInto;

// TODO: This class will include the indexes of 3 matrices
// Should domain info be in here or in a separate class?
use math::{fft, utils, FieldElement, StarkField};

type SmallFieldElement17 = math::fields::smallprimefield::BaseElement<17, 3, 4>;

use crate::indexed_matrix::IndexedMatrix;
use models::r1cs::R1CS;

#[derive(Clone, Debug)]
pub struct IndexParams {
    pub num_input_variables: usize,
    // num_witness_variables: usize,
    pub num_constraints: usize,
    pub num_non_zero: usize,
}
#[derive(Clone, Debug)]
pub struct Index<E: StarkField> {
    pub params: IndexParams,
    pub indexed_a: IndexedMatrix<E>,
    pub indexed_b: IndexedMatrix<E>,
    pub indexed_c: IndexedMatrix<E>,
}

impl<E: StarkField> Index<E> {
    pub fn new(
        params: IndexParams,
        indexed_a: IndexedMatrix<E>,
        indexed_b: IndexedMatrix<E>,
        indexed_c: IndexedMatrix<E>,
    ) -> Self {
        Index {
            params,
            indexed_a: indexed_a,
            indexed_b: indexed_b,
            indexed_c: indexed_c,
        }
    }
}

/// QUESTION: Currently IndexDomains is implemented over a generic FieldElement trait.
/// but do we want to keep it this way, since below the actual implementation to generate
/// indices is BaseElement
#[derive(Clone, Debug)]
pub struct IndexDomains<E: FieldElement> {
    pub i_field_base: E,
    pub h_field_base: E,
    pub k_field_base: E,
    pub l_field_base: E,
    pub i_field: Vec<E>,
    pub h_field: Vec<E>,
    pub k_field_len: usize,
    pub l_field_len: usize,
    pub inv_twiddles_k_elts: Vec<E>,
    pub twiddles_l_elts: Vec<E>,
}

/// ***************  HELPERS *************** \\\

// Currently assuming that
// 1. All the inputs to this function are powers of 2
// 2. num_input_variables is the number of inputs and num_input_variables + num_witnesses = num_constraints
// 3. 2, above implies that the matrices are all square.
/// QUESTION: This is currently built using BaseField because the trait has no generic function for
/// getting generators of a certain order. I think this would require some re-structuring.
/// Perhaps we can add a function "get_subgroup_of_size" or "get_generator_of_order"
/// Generators are needed here since we'll need those for FFT-friendly subgroups anyway.
pub fn build_index_domains<E: StarkField>(params: IndexParams) -> IndexDomains<E> {
    let num_input_variables = params.num_input_variables;
    let num_constraints = params.num_constraints;
    let num_non_zero = params.num_non_zero;

    // Validate inputs.
    let ntpow2 = { |x: usize| x > 1 && (x & (x - 1) == 0) };
    assert!(
        ntpow2(num_input_variables),
        "num_input_variables {} must be nontriv power of two",
        num_input_variables
    );
    assert!(
        ntpow2(num_constraints),
        "num_constraints {} must be nontriv power of two",
        num_constraints
    );
    assert!(
        ntpow2(num_non_zero),
        "num_non_zero {} must be nontriv power of two",
        num_non_zero
    );

    // Set up the needed field elements.
    let i_field_base = E::get_root_of_unity(num_input_variables.trailing_zeros());
    let h_field_base = E::get_root_of_unity(num_constraints.trailing_zeros());
    let k_field_base = E::get_root_of_unity(num_non_zero.trailing_zeros());
    let ext_field_size = 4 * num_non_zero; // this should actually be 3*k_field_size - 3 but will change later.
    let l_field_base = E::get_root_of_unity(ext_field_size.trailing_zeros());
    let i_field = utils::get_power_series(i_field_base, num_input_variables);
    let h_field = utils::get_power_series(h_field_base, num_constraints);

    // Prepare the FFT coefficients (twiddles).

    // let inv_twiddles_k_elts = fft::get_inv_twiddles(k_field_base, num_non_zero);
    // let twiddles_l_elts = fft::get_twiddles(l_field_base, ext_field_size);

    let inv_twiddles_k_elts = fft::get_inv_twiddles::<E>(num_non_zero);
    let twiddles_l_elts = fft::get_twiddles::<E>(ext_field_size);

    IndexDomains {
        i_field_base,
        h_field_base,
        k_field_base,
        l_field_base,
        i_field,
        h_field,
        k_field_len: num_non_zero,
        l_field_len: ext_field_size,
        inv_twiddles_k_elts,
        twiddles_l_elts,
    }
}

// Same as build_basefield_index_domains but for a prime field of size 17
pub fn build_primefield_index_domains(params: IndexParams) -> IndexDomains<SmallFieldElement17> {
    let num_input_variables = params.num_input_variables;
    let num_constraints = params.num_constraints;
    let num_non_zero = params.num_non_zero;
    let i_field_base =
        SmallFieldElement17::get_root_of_unity(num_input_variables.try_into().unwrap());
    let h_field_base = SmallFieldElement17::get_root_of_unity(num_constraints.try_into().unwrap());
    let k_field_base = SmallFieldElement17::get_root_of_unity(num_non_zero.try_into().unwrap());
    let ext_field_size = 4 * num_non_zero; // this should actually be 3*k_field_size - 3 but will change later.
    let l_field_base = SmallFieldElement17::get_root_of_unity(ext_field_size.try_into().unwrap());
    unsafe {
        let i_field = SmallFieldElement17::get_power_series(i_field_base, num_input_variables);
        let h_field = SmallFieldElement17::get_power_series(h_field_base, num_constraints);

        // let inv_twiddles_k_elts = fft::get_inv_twiddles(k_field_base, num_non_zero);
        // let twiddles_l_elts = fft::get_twiddles(l_field_base, ext_field_size);
        let inv_twiddles_k_elts = SmallFieldElement17::get_inv_twiddles(num_non_zero);
        let twiddles_l_elts = SmallFieldElement17::get_twiddles(ext_field_size);

        IndexDomains {
            i_field_base,
            h_field_base,
            k_field_base,
            l_field_base,
            i_field,
            h_field,
            k_field_len: num_non_zero,
            l_field_len: ext_field_size,
            inv_twiddles_k_elts,
            twiddles_l_elts,
        }
    }
}

// TODO Update the new function for Index to take an R1CS instance as input.

pub fn create_index_from_r1cs<E: StarkField>(
    params: IndexParams,
    r1cs_instance: R1CS<E>,
) -> Index<E> {
    let domains = build_index_domains(params.clone());
    let indexed_a = IndexedMatrix::new(&r1cs_instance.A, &domains);
    let indexed_b = IndexedMatrix::new(&r1cs_instance.B, &domains);
    let indexed_c = IndexedMatrix::new(&r1cs_instance.C, &domains);
    Index::new(params, indexed_a, indexed_b, indexed_c)
}

pub fn create_primefield_index_from_r1cs(
    params: IndexParams,
    r1cs_instance: R1CS<SmallFieldElement17>,
) -> Index<SmallFieldElement17> {
    let domains = build_primefield_index_domains(params.clone());
    let indexed_a = IndexedMatrix::new(&r1cs_instance.A, &domains);
    let indexed_b = IndexedMatrix::new(&r1cs_instance.B, &domains);
    let indexed_c = IndexedMatrix::new(&r1cs_instance.C, &domains);
    Index::new(params, indexed_a, indexed_b, indexed_c)
}
