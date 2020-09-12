use super::TraceTable;
use crate::ConstraintEvaluator;

pub fn evaluate_constraints<E: ConstraintEvaluator>(
    evaluator: &E,
    trace: &TraceTable,
    _lde_domain: &Vec<u128>,
) {
    let stride = evaluator.blowup_factor() / E::MAX_CONSTRAINT_DEGREE;
    for _i in (0..trace.num_states()).step_by(stride) {
        /*
        let current = vec![];
        let next = vec![];
        t_evaluations[i] =
            evaluator
                .evaluate_transition(&current, &next, lde_domain[i], i / stride);
        b_evaluations[i] = self.evaluator.evaluate_boundaries(&current, lde_domain[i]);
        */
    }
}
