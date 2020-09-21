mod options;
pub use options::ProofOptions;

mod proof;
pub use proof::{DeepValues, FriLayer, FriProof, StarkProof};

mod trace_info;
pub use trace_info::TraceInfo;

mod composition;
pub use composition::{draw_z_and_coefficients, CompositionCoefficients};

mod evaluator;
pub use evaluator::{
    Assertion, AssertionEvaluator, ConstraintDivisor, ConstraintEvaluator, IoAssertionEvaluator,
    TransitionEvaluator,
};

mod queries;
pub use queries::compute_trace_query_positions;
