use crate::{ field, polynom };

#[test]
fn fft_in_place() {
    // degree 3
    let mut p: [u128; 4] = [1, 2, 3, 4];
    let g = field::get_root_of_unity(4);
    let xs = field::get_power_series(g, 4);
    let expected: Vec<u128> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    let twiddles = super::get_twiddles(g, 4);
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 7
    let mut p: [u128; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let g = field::get_root_of_unity(8);
    let twiddles = super::get_twiddles(g, 8);
    let xs = field::get_power_series(g, 8);
    let expected: Vec<u128> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 15
    let mut p: [u128; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let g = field::get_root_of_unity(16);
    let twiddles = super::get_twiddles(g, 16);
    let xs = field::get_power_series(g, 16);
    let expected: Vec<u128> = xs.into_iter().map(|x| polynom::eval(&p, x)).collect();
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);

    // degree 1023
    let mut p = field::rand_vector(1024);
    let g = field::get_root_of_unity(1024);
    let roots = field::get_power_series(g, 1024);
    let expected = roots.iter().map(|x| polynom::eval(&p, *x)).collect::<Vec<u128>>();
    let twiddles = super::get_twiddles(g, 1024);
    super::fft_in_place(&mut p, &twiddles, 1, 1, 0);
    super::permute(&mut p);
    assert_eq!(expected, p);
}
