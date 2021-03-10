use crate::{
    field::{BaseElement, FieldElement, StarkField},
    polynom,
};

#[test]
fn fft_evaluate_poly() {
    let n = super::MIN_CONCURRENT_SIZE * 2;
    let mut p = build_random_element_vec(n);

    let xs = build_domain(n);
    let expected: Vec<BaseElement> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();

    let twiddles = build_twiddles(n);
    super::evaluate_poly(&mut p, &twiddles);
    assert_eq!(expected, p);
}

#[test]
fn fft_interpolate_poly() {
    let n = super::MIN_CONCURRENT_SIZE * 2;
    let expected: Vec<BaseElement> = build_random_element_vec(n);

    let xs = build_domain(n);
    let mut ys: Vec<BaseElement> = xs
        .into_iter()
        .map(|x| polynom::eval(&expected, x))
        .collect();

    let inv_twiddles = build_inv_twiddles(n);
    super::interpolate_poly(&mut ys, &inv_twiddles);
    assert_eq!(expected, ys);
}

#[test]
fn fft_in_place() {
    // degree 3
    let mut p: [BaseElement; 4] = [
        BaseElement::from(1u8),
        BaseElement::from(2u8),
        BaseElement::from(3u8),
        BaseElement::from(4u8),
    ];
    let g = BaseElement::get_root_of_unity(2);
    let xs = BaseElement::get_power_series(g, 4);
    let expected: Vec<BaseElement> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    let twiddles = super::get_twiddles(g, 4);
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 7
    let mut p: Vec<BaseElement> = vec![1u8, 2, 3, 4, 5, 6, 7, 8]
        .into_iter()
        .map(BaseElement::from)
        .collect();
    let g = BaseElement::get_root_of_unity(3);
    let twiddles = super::get_twiddles(g, 8);
    let xs = BaseElement::get_power_series(g, 8);
    let expected: Vec<BaseElement> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 15
    let mut p: Vec<BaseElement> = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        .into_iter()
        .map(BaseElement::from)
        .collect();
    let g = BaseElement::get_root_of_unity(4);
    let twiddles = super::get_twiddles(g, 16);
    let xs = BaseElement::get_power_series(g, 16);
    let expected: Vec<BaseElement> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 1023
    let mut p = BaseElement::prng_vector(build_seed(), 1024);
    let g = BaseElement::get_root_of_unity(10);
    let roots = BaseElement::get_power_series(g, 1024);
    let expected = roots
        .iter()
        .map(|x| polynom::eval(&p, *x))
        .collect::<Vec<BaseElement>>();
    let twiddles = super::get_twiddles(g, 1024);
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);
}

#[test]
fn fft_get_twiddles() {
    let n = super::MIN_CONCURRENT_SIZE * 2;
    let g = BaseElement::get_root_of_unity(n.trailing_zeros());

    let mut expected = BaseElement::get_power_series(g, n / 2);
    super::permute_values(&mut expected);

    let twiddles = super::get_twiddles(g, n);
    assert_eq!(expected, twiddles);
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_seed() -> [u8; 32] {
    let mut result = [0; 32];
    let seed = BaseElement::rand().to_bytes();
    result[..16].copy_from_slice(&seed);
    result
}

fn build_random_element_vec(size: usize) -> Vec<BaseElement> {
    BaseElement::prng_vector(build_seed(), size)
}

fn build_domain(size: usize) -> Vec<BaseElement> {
    let g = BaseElement::get_root_of_unity(size.trailing_zeros());
    BaseElement::get_power_series(g, size)
}

fn build_twiddles(size: usize) -> Vec<BaseElement> {
    let g = BaseElement::get_root_of_unity(size.trailing_zeros());
    super::get_twiddles(g, size)
}

fn build_inv_twiddles(size: usize) -> Vec<BaseElement> {
    let g = BaseElement::get_root_of_unity(size.trailing_zeros());
    super::get_inv_twiddles(g, size)
}
