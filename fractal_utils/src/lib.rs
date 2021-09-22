pub mod errors;
pub mod matrix_utils;
pub mod polynomial_utils;
#[cfg(test)]
mod tests;
pub type SmallFieldElement17 = math::fields::smallprimefield::BaseElement<17, 3>;
pub type SmallFieldElement13 = math::fields::smallprimefield::BaseElement<13, 2>;
