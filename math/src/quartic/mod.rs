use crate::{
    field::{f128::FieldElement, StarkField},
    utils::uninit_vector,
};

#[cfg(test)]
mod tests;

/// Evaluates degree 3 polynomial `p` at coordinate `x`. This function is about 30% faster than
/// the `polynom::eval` function.
pub fn eval(p: &[FieldElement], x: FieldElement) -> FieldElement {
    debug_assert!(p.len() == 4, "Polynomial must have 4 terms");
    let mut y = p[0] + p[1] * x;

    let x2 = x * x;
    y = y + p[2] * x2;

    let x3 = x2 * x;
    y = y + p[3] * x3;

    y
}

/// Evaluates a batch of degree 3 polynomials at the provided X coordinate.
pub fn evaluate_batch(polys: &[[FieldElement; 4]], x: FieldElement) -> Vec<FieldElement> {
    let n = polys.len();

    let mut result: Vec<FieldElement> = Vec::with_capacity(n);
    unsafe {
        result.set_len(n);
    }

    for i in 0..n {
        result[i] = eval(&polys[i], x);
    }

    result
}

/// Interpolates a set of X, Y coordinates into a batch of degree 3 polynomials.
///
/// This function is many times faster than using `polynom::interpolate` function in a loop.
/// This is primarily due to amortizing inversions over the entire batch.
pub fn interpolate_batch(
    xs: &[[FieldElement; 4]],
    ys: &[[FieldElement; 4]],
) -> Vec<[FieldElement; 4]> {
    debug_assert!(
        xs.len() == ys.len(),
        "number of X coordinates must be equal to number of Y coordinates"
    );

    let n = xs.len();
    let mut equations: Vec<[FieldElement; 4]> = Vec::with_capacity(n * 4);
    let mut inverses: Vec<FieldElement> = Vec::with_capacity(n * 4);
    unsafe {
        equations.set_len(n * 4);
        inverses.set_len(n * 4);
    }

    for (i, j) in (0..n).zip((0..equations.len()).step_by(4)) {
        let xs = xs[i];

        let x01 = xs[0] * xs[1];
        let x02 = xs[0] * xs[2];
        let x03 = xs[0] * xs[3];
        let x12 = xs[1] * xs[2];
        let x13 = xs[1] * xs[3];
        let x23 = xs[2] * xs[3];

        // eq0
        equations[j] = [
            -x12 * xs[3],
            x12 + x13 + x23,
            -xs[1] - xs[2] - xs[3],
            FieldElement::ONE,
        ];
        inverses[j] = eval(&equations[j], xs[0]);

        // eq1
        equations[j + 1] = [
            -x02 * xs[3],
            x02 + x03 + x23,
            -xs[0] - xs[2] - xs[3],
            FieldElement::ONE,
        ];
        inverses[j + 1] = eval(&equations[j + 1], xs[1]);

        // eq2
        equations[j + 2] = [
            -x01 * xs[3],
            x01 + x03 + x13,
            -xs[0] - xs[1] - xs[3],
            FieldElement::ONE,
        ];
        inverses[j + 2] = eval(&equations[j + 2], xs[2]);

        // eq3
        equations[j + 3] = [
            -x01 * xs[2],
            x01 + x02 + x12,
            -xs[0] - xs[1] - xs[2],
            FieldElement::ONE,
        ];
        inverses[j + 3] = eval(&equations[j + 3], xs[3]);
    }

    let inverses = FieldElement::inv_many(&inverses);

    let mut result: Vec<[FieldElement; 4]> = Vec::with_capacity(n);
    unsafe {
        result.set_len(n);
    }

    for (i, j) in (0..n).zip((0..equations.len()).step_by(4)) {
        let ys = ys[i];

        // iteration 0
        let mut inv_y = ys[0] * inverses[j];
        result[i][0] = inv_y * equations[j][0];
        result[i][1] = inv_y * equations[j][1];
        result[i][2] = inv_y * equations[j][2];
        result[i][3] = inv_y * equations[j][3];

        // iteration 1
        inv_y = ys[1] * inverses[j + 1];
        result[i][0] = result[i][0] + inv_y * equations[j + 1][0];
        result[i][1] = result[i][1] + inv_y * equations[j + 1][1];
        result[i][2] = result[i][2] + inv_y * equations[j + 1][2];
        result[i][3] = result[i][3] + inv_y * equations[j + 1][3];

        // iteration 2
        inv_y = ys[2] * inverses[j + 2];
        result[i][0] = result[i][0] + inv_y * equations[j + 2][0];
        result[i][1] = result[i][1] + inv_y * equations[j + 2][1];
        result[i][2] = result[i][2] + inv_y * equations[j + 2][2];
        result[i][3] = result[i][3] + inv_y * equations[j + 2][3];

        // iteration 3
        inv_y = ys[3] * inverses[j + 3];
        result[i][0] = result[i][0] + inv_y * equations[j + 3][0];
        result[i][1] = result[i][1] + inv_y * equations[j + 3][1];
        result[i][2] = result[i][2] + inv_y * equations[j + 3][2];
        result[i][3] = result[i][3] + inv_y * equations[j + 3][3];
    }

    result
}

pub fn transpose(vector: &[FieldElement], stride: usize) -> Vec<[FieldElement; 4]> {
    assert!(
        vector.len() % (4 * stride) == 0,
        "vector length must be divisible by {}",
        4 * stride
    );
    let row_count = vector.len() / (4 * stride);

    let mut result = to_quartic_vec(uninit_vector(row_count * 4));
    for i in 0..row_count {
        result[i] = [
            vector[i * stride],
            vector[(i + row_count) * stride],
            vector[(i + 2 * row_count) * stride],
            vector[(i + 3 * row_count) * stride],
        ];
    }

    result
}

/// Re-interprets a vector of integers as a vector of quartic elements.
pub fn to_quartic_vec(vector: Vec<FieldElement>) -> Vec<[FieldElement; 4]> {
    assert!(
        vector.len() % 4 == 0,
        "vector length must be divisible by 4"
    );
    let mut v = std::mem::ManuallyDrop::new(vector);
    let p = v.as_mut_ptr();
    let len = v.len() / 4;
    let cap = v.capacity() / 4;
    unsafe { Vec::from_raw_parts(p as *mut [FieldElement; 4], len, cap) }
}
