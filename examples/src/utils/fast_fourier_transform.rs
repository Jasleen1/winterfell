use winterfell::math::{fields::f128::BaseElement, log2, FieldElement};

// Implements a simple iterative FFT using the Cooley-Tukey algorithm from
// https://en.wikipedia.org/wiki/Cooley–Tukey_FFT_algorithm#Data_reordering,_bit_reversal,_and_in-place_algorithms
pub(crate) fn simple_iterative_fft(
    input_array: Vec<BaseElement>,
    omega: BaseElement,
) -> Vec<BaseElement> {
    let mut output_arr = bit_reverse_copy(input_array.clone());
    let fft_size = input_array.len();
    let log_fft_size = log2(fft_size);
    let num_steps: usize = log_fft_size.try_into().unwrap();
    let fft_size_u128: u128 = fft_size.try_into().unwrap();
    for step in 1..num_steps + 1 {
        let m = 1 << step;
        let omega_pow = fft_size_u128 / m;
        let local_factor = omega.exp(omega_pow);
        let k_upperbound: usize = (fft_size_u128 / m).try_into().unwrap();
        let jump: usize = (m / 2).try_into().unwrap();
        for k in 0..k_upperbound {
            let mut omega_curr = BaseElement::ONE;
            let start_pos = k * jump * 2;
            for j in 0..jump {
                let u = output_arr[start_pos + j];
                let v = omega_curr * output_arr[start_pos + j + jump];
                output_arr[start_pos + j] = u + v;
                output_arr[start_pos + j + jump] = u - v;
                omega_curr = omega_curr * local_factor;
            }
        }
    }
    output_arr
}

fn bit_reverse_copy(input_array: Vec<BaseElement>) -> Vec<BaseElement> {
    let mut output_arr = input_array.clone();
    let fft_size = input_array.len();
    let log_fft_size = log2(fft_size);
    let num_bits: usize = log_fft_size.try_into().unwrap();
    for i in 0..fft_size {
        output_arr[bit_reverse(i, num_bits)] = input_array[i];
    }
    output_arr
}

// bit_reverse(0xb1011, 4) -> 0xb1101
// bit_reverse(0xb1011, 4) -> 0xb1101
pub(crate) fn bit_reverse(input_int: usize, num_bits: usize) -> usize {
    let mut output_int = 0;
    let mut input_copy = input_int;
    for _ in 0..num_bits {
        output_int <<= 1;
        output_int |= input_copy & 1;
        input_copy >>= 1;
    }
    return output_int;
}
