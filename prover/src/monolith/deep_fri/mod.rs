use super::{constraints::ConstraintPoly, types, utils};

mod composition;
pub use composition::{compose_constraint_poly, compose_trace_polys};

mod composition_poly;
pub use composition_poly::CompositionPoly;
