use crate::{
    field::{BaseElement, FieldElement, StarkField},
    polynom,
};

#[test]
fn fft_poly_eval() {
    let mut p: Vec<BaseElement> = (1u8..33)
        .map(BaseElement::from)
        .collect();
    let g = BaseElement::get_root_of_unity(p.len().trailing_zeros());
    println!("g: {}", g);
    let xs = BaseElement::get_power_series(g, p.len());
    let expected: Vec<BaseElement> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    let twiddles = super::get_twiddles(g, p.len());
    super::evaluate_poly(&mut p, &twiddles);
    assert_eq!(expected, p);
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

// HELPER FUNCTIONS
// ================================================================================================
fn build_seed() -> [u8; 32] {
    let mut result = [0; 32];
    let seed = BaseElement::rand().to_bytes();
    result[..16].copy_from_slice(&seed);
    result
}
