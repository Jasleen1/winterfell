use super::{ComputationContext, ConstraintDegree, RandomGenerator};
use math::field::{FieldElement, StarkField};
use std::collections::HashMap;

// TRANSITION EVALUATOR TRAIT
// ================================================================================================

pub trait TransitionEvaluator {
    fn new(context: &ComputationContext, coeff_prng: RandomGenerator) -> Self;

    // ABSTRACT METHODS
    // --------------------------------------------------------------------------------------------

    /// Evaluates transition constraints at the specified `step` of the execution trace extended
    /// over constraint evaluation domain. The evaluations are saved into the `results` slice. This
    /// method is used by the prover to evaluate/ constraint for all steps of the execution trace.
    fn evaluate_at_step(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        step: usize,
    );

    /// Evaluates transition constraints at the specified `x` coordinate, which could be in or out
    /// of evaluation domain. The evaluations are saved into the `results` slice. This method is
    /// used by both the prover and the verifier to evaluate constraints at an out-of-domain point.
    fn evaluate_at_x(
        &self,
        result: &mut [FieldElement],
        current: &[FieldElement],
        next: &[FieldElement],
        x: FieldElement,
    );

    /// Returns constraints grouped by their degree.
    fn constraint_groups(&self) -> &[TransitionConstraintGroup];

    /// Returns constraint evaluation domain blowup factor required for evaluating
    /// transition constraints defined by this evaluator.
    fn get_ce_blowup_factor() -> usize;

    // IMPLEMENTED METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns number of transition constraints defined for this evaluator.
    fn num_constraints(&self) -> usize {
        let mut result = 0;
        for group in self.constraint_groups() {
            result += group.indexes.len()
        }
        result
    }

    /// Returns degrees of all individual transition constraints.
    fn get_constraint_degrees(&self) -> Vec<ConstraintDegree> {
        let mut degrees = vec![ConstraintDegree::new(1); self.num_constraints()];
        for group in self.constraint_groups() {
            for &index in group.indexes.iter() {
                degrees[index] = group.degree.clone()
            }
        }
        degrees
    }

    /// Merges all transition constraint evaluations into a single value; we can do this
    /// because all transition constraint evaluations have the same divisor, and this
    /// divisor will be applied later to this single value.
    fn merge_evaluations(&self, evaluations: &[FieldElement], x: FieldElement) -> FieldElement {
        let mut result = FieldElement::ZERO;

        for group in self.constraint_groups() {
            // for each group of constraints with the same degree, separately compute
            // combinations of D(x) and D(x) * x^p
            let mut result_adj = FieldElement::ZERO;
            for (&constraint_idx, coefficients) in
                group.indexes.iter().zip(group.coefficients.iter())
            {
                let evaluation = evaluations[constraint_idx];
                result = result + evaluation * coefficients.0;
                result_adj = result_adj + evaluation * coefficients.1;
            }

            // increase the degree of D(x) * x^p
            let xp = FieldElement::exp(x, group.degree_adjustment);
            result = result + result_adj * xp;
        }

        result
    }

    /// Groups transition constraints together by their degree, and also assigns coefficients
    /// to each constraint. These coefficients will be used to compute random linear combination
    /// of transition constraints during constraint merging.
    fn group_constraints(
        context: &ComputationContext,
        degrees: &[ConstraintDegree],
        mut coeff_prng: RandomGenerator,
    ) -> Vec<TransitionConstraintGroup> {
        // We want to make sure that once we divide constraint polynomials by the divisor,
        // the degree of the resulting polynomial will be exactly equal to the composition_degree.
        // For transition constraints, divisor degree = deg(trace). So, target degree for all
        // transitions constraints is simply: deg(composition) + deg(trace)
        let divisor_degree = context.trace_length() - 1;
        let target_degree = context.composition_degree() + divisor_degree;

        let mut groups = HashMap::new();
        for (i, degree) in degrees.iter().enumerate() {
            let evaluation_degree = degree.get_evaluation_degree(context.trace_length());
            let degree_adjustment = (target_degree - evaluation_degree) as u128;
            let group = groups
                .entry(evaluation_degree)
                .or_insert(TransitionConstraintGroup {
                    degree: degree.clone(),
                    degree_adjustment,
                    indexes: Vec::new(),
                    coefficients: Vec::new(),
                });
            group.indexes.push(i);
            group.coefficients.push(coeff_prng.draw_pair());
        }

        groups.into_iter().map(|e| e.1).collect()
    }
}

// TRANSITION CONSTRAINT GROUP
// ================================================================================================

pub struct TransitionConstraintGroup {
    pub degree: ConstraintDegree,
    pub degree_adjustment: u128,
    pub indexes: Vec<usize>,
    pub coefficients: Vec<(FieldElement, FieldElement)>,
}
