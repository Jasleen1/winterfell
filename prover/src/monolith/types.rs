use math::{
    field::{BaseElement, FieldElement, FromVec},
    polynom,
};

// POLYNOMIAL TABLE
// ================================================================================================
pub struct PolyTable(Vec<Vec<BaseElement>>);

impl PolyTable {
    pub fn new(polys: Vec<Vec<BaseElement>>) -> Self {
        assert!(
            !polys.is_empty(),
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
    pub fn evaluate_at<E: FieldElement + FromVec<BaseElement>>(&self, x: E) -> Vec<E> {
        let mut result = Vec::with_capacity(self.num_polys());
        for poly in self.0.iter() {
            result.push(polynom::eval(&E::from_vec(&poly), x));
        }
        result
    }

    pub fn num_polys(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub fn get_poly(&self, idx: usize) -> &[BaseElement] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<BaseElement>> {
        self.0
    }
}
