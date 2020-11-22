use super::{
    Assertion, AssertionEvaluator, ComputationContext, ConstraintDivisor, RandomGenerator,
};
use crate::errors::EvaluatorError;
use math::field::{BaseElement, FieldElement};
use std::collections::BTreeMap;

// DEFAULT ASSERTION EVALUATOR
// ================================================================================================

/// Default assertion evaluator enables assertions against arbitrary steps and registers in the
/// execution trace. However, each assertion becomes a separate constraint, and constraints are
/// grouped by execution step. Thus, using this evaluator to make assertions against a large number
/// of steps may be inefficient.
pub struct DefaultAssertionEvaluator {
    constraint_groups: Vec<AssertionConstraintGroup>,
    divisors: Vec<ConstraintDivisor>,
}

impl AssertionEvaluator for DefaultAssertionEvaluator {
    fn new(
        context: &ComputationContext,
        assertions: &[Assertion],
        coeff_prng: RandomGenerator,
    ) -> Result<Self, EvaluatorError> {
        let constraint_groups = group_assertions(context, assertions, coeff_prng)?;
        Ok(DefaultAssertionEvaluator {
            divisors: constraint_groups
                .iter()
                .map(|c| c.divisor.clone())
                .collect(),
            constraint_groups,
        })
    }

    fn evaluate<E: FieldElement<PositiveInteger = u128> + From<BaseElement>>(
        &self,
        result: &mut [E],
        state: &[E],
        x: E,
    ) {
        let mut degree_adjustment = self.constraint_groups[0].degree_adjustment;
        let mut xp = E::exp(x, degree_adjustment);

        for (i, group) in self.constraint_groups.iter().enumerate() {
            if group.degree_adjustment != degree_adjustment {
                degree_adjustment = group.degree_adjustment;
                xp = E::exp(x, degree_adjustment);
            }
            result[i] = group.evaluate(state, xp);
        }
    }

    fn divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }
}

// ASSERTION CONSTRAINT
// ================================================================================================

#[derive(Debug, Clone)]
struct AssertionConstraint {
    register: usize,
    value: BaseElement,
}

// ASSERTION CONSTRAINT GROUP
// ================================================================================================

/// A group of assertion constraints all having the same divisor.
#[derive(Debug, Clone)]
struct AssertionConstraintGroup {
    constraints: Vec<AssertionConstraint>,
    coefficients: Vec<(BaseElement, BaseElement)>,
    divisor: ConstraintDivisor,
    degree_adjustment: u128,
}

impl AssertionConstraintGroup {
    fn new(context: &ComputationContext, divisor: ConstraintDivisor) -> Self {
        // We want to make sure that once we divide a constraint polynomial by its divisor, the
        // degree of the resulting polynomials will be exactly equal to the composition_degree.
        // Assertion constraint degree is always deg(trace). So, the adjustment degree is simply:
        // deg(composition) + deg(divisor) - deg(trace)
        let target_degree = context.composition_degree() + divisor.degree();
        let degree_adjustment = (target_degree - context.trace_poly_degree()) as u128;

        AssertionConstraintGroup {
            constraints: Vec::new(),
            coefficients: Vec::new(),
            divisor,
            degree_adjustment,
        }
    }

    fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], xp: E) -> E {
        let mut result = E::ZERO;
        let mut result_adj = E::ZERO;

        for (constraint, coefficients) in self.constraints.iter().zip(self.coefficients.iter()) {
            let value = state[constraint.register] - E::from(constraint.value);
            result = result + value * E::from(coefficients.0);
            result_adj = result_adj + value * E::from(coefficients.1);
        }

        result + result_adj * xp
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn group_assertions(
    context: &ComputationContext,
    assertions: &[Assertion],
    mut coeff_prng: RandomGenerator,
) -> Result<Vec<AssertionConstraintGroup>, EvaluatorError> {
    // use BTreeMap to make sure assertions are always grouped in consistent order
    let mut groups = BTreeMap::new();

    // iterate over all assertions and group them by step - i.e.: assertions for the first
    // step are grouped together, assertions for the last step are grouped together etc.
    for assertion in assertions {
        if assertion.register() >= context.trace_width() {
            return Err(EvaluatorError::InvalidAssertionRegisterIndex(
                assertion.register(),
            ));
        }
        if assertion.step() >= context.trace_length() {
            return Err(EvaluatorError::InvalidAssertionStep(assertion.step()));
        }

        // get a group for the assertion step, or create one if one doesn't exist yet
        let group = groups.entry(assertion.step()).or_insert_with(|| {
            let divisor =
                ConstraintDivisor::from_assertion(context.get_trace_x_at(assertion.step()));
            AssertionConstraintGroup::new(context, divisor)
        });

        // add assertion to the group using binary search; this makes sure that
        // assertions are always sorted in consistent order
        match group
            .constraints
            .binary_search_by_key(&assertion.register(), |c| c.register)
        {
            Ok(_) => {
                return Err(EvaluatorError::DuplicateAssertion(
                    assertion.register(),
                    assertion.step(),
                ))
            }
            Err(pos) => group.constraints.insert(
                pos,
                AssertionConstraint {
                    register: assertion.register(),
                    value: assertion.value(),
                },
            ),
        }

        // add coefficients for the assertion (two coefficients per assertion); these coefficients
        // will be used to compute random linear combination of constraint evaluations
        group.coefficients.push(coeff_prng.draw_pair());
    }

    // make sure groups are sorted by adjustment degree
    let mut groups: Vec<AssertionConstraintGroup> =
        groups.into_iter().map(|entry| entry.1).collect();
    groups.sort_by_key(|c| c.degree_adjustment);

    Ok(groups)
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use crate::{Assertion, ComputationContext, ProofOptions, RandomGenerator};
    use crypto::hash::blake3;
    use math::field::BaseElement;

    #[test]
    fn group_assertions() {
        let options = ProofOptions::new(32, 4, 0, blake3);
        let context = ComputationContext::new(2, 8, 2, options);

        let groups = super::group_assertions(
            &context,
            &[
                Assertion::new(1, 4, BaseElement::new(4)),
                Assertion::new(1, 0, BaseElement::new(2)),
                Assertion::new(0, 0, BaseElement::new(1)),
                Assertion::new(0, 4, BaseElement::new(3)),
                Assertion::new(0, 7, BaseElement::new(5)),
            ],
            RandomGenerator::new([0; 32], 0, blake3),
        )
        .unwrap();

        let group1 = &groups[0];
        assert_eq!(group1.constraints[0].register, 0);
        assert_eq!(group1.constraints[0].value, BaseElement::new(1));
        assert_eq!(group1.constraints[1].register, 1);
        assert_eq!(group1.constraints[1].value, BaseElement::new(2));
        // TODO: check divisor

        let group2 = &groups[1];
        assert_eq!(group2.constraints[0].register, 0);
        assert_eq!(group2.constraints[0].value, BaseElement::new(3));
        assert_eq!(group2.constraints[1].register, 1);
        assert_eq!(group2.constraints[1].value, BaseElement::new(4));
        // TODO: check divisor

        let group3 = &groups[2];
        assert_eq!(group3.constraints[0].register, 0);
        assert_eq!(group3.constraints[0].value, BaseElement::new(5));
        // TODO: check divisor
    }
}
