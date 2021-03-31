use math::{
    field::{BaseElement, FieldElement},
    polynom,
};

// POLYNOMIAL TABLE
// ================================================================================================
pub struct TracePolyTable(Vec<Vec<BaseElement>>);

impl TracePolyTable {
    /// Creates a new table of trace polynomials from the provided vectors.
    pub fn new(polys: Vec<Vec<BaseElement>>) -> Self {
        assert!(
            !polys.is_empty(),
            "trace polynomial table must contain at least one polynomial"
        );
        let poly_size = polys[0].len();
        assert!(
            poly_size.is_power_of_two(),
            "trace polynomial size must be a power of 2"
        );
        for poly in polys.iter() {
            assert!(
                poly.len() == poly_size,
                "all trace polynomials must have the same size"
            );
        }

        TracePolyTable(polys)
    }

    /// Returns the size of each polynomial - i.e. size of a vector needed to hold a polynomial.
    pub fn poly_size(&self) -> usize {
        self.0[0].len()
    }

    /// Evaluates all trace polynomials the the specified point `x`.
    pub fn evaluate_at<E: FieldElement + From<BaseElement>>(&self, x: E) -> Vec<E> {
        self.0.iter().map(|p| polynom::eval(p, x)).collect()
    }

    /// Returns the number of trace polynomials in the table.
    pub fn num_polys(&self) -> usize {
        self.0.len()
    }

    /// Returns a trace polynomial at the specified index.
    #[cfg(test)]
    pub fn get_poly(&self, idx: usize) -> &[BaseElement] {
        &self.0[idx]
    }

    /// Converts this table into a vector of polynomials.
    pub fn into_vec(self) -> Vec<Vec<BaseElement>> {
        self.0
    }
}
