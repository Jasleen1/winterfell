use super::{
    Assertion, AssertionConstraint, AssertionConstraintGroup, AssertionEvaluator,
    ConstraintDivisor, ProofContext,
};
use crate::errors::EvaluatorError;
use math::field::{FieldElement, StarkField};
use std::collections::BTreeMap;

// INPUT/OUTPUT ASSERTION EVALUATOR
// ================================================================================================

pub struct BasicAssertionEvaluator {
    constraints: Vec<AssertionConstraintGroup>,
    divisors: Vec<ConstraintDivisor>,
}

impl AssertionEvaluator for BasicAssertionEvaluator {
    const MAX_CONSTRAINTS: usize = 128;

    fn new(
        context: &ProofContext,
        assertions: &[Assertion],
        coefficients: &[FieldElement],
    ) -> Result<Self, EvaluatorError> {
        let constraints = group_assertions(context, assertions, coefficients)?;
        Ok(BasicAssertionEvaluator {
            divisors: constraints.iter().map(|c| c.divisor.clone()).collect(),
            constraints,
        })
    }

    fn evaluate(&self, result: &mut [FieldElement], state: &[FieldElement], x: FieldElement) {
        let mut degree_adjustment = self.constraints[0].degree_adjustment;
        let mut xp = FieldElement::exp(x, degree_adjustment);

        for (i, group) in self.constraints.iter().enumerate() {
            if self.constraints[i].degree_adjustment != degree_adjustment {
                degree_adjustment = self.constraints[i].degree_adjustment;
                xp = FieldElement::exp(x, degree_adjustment);
            }
            result[i] = group.evaluate(state, xp);
        }
    }

    fn divisors(&self) -> &[ConstraintDivisor] {
        &self.divisors
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn group_assertions(
    context: &ProofContext,
    assertions: &[Assertion],
    coefficients: &[FieldElement],
) -> Result<Vec<AssertionConstraintGroup>, EvaluatorError> {
    // use BTreeMap to make sure assertions are always grouped in consistent order
    let mut groups = BTreeMap::new();
    let mut i = 0;

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
        group
            .coefficients
            .push((coefficients[i], coefficients[i + 1]));
        i += 2;
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

    use crate::stark::{Assertion, ProofContext, ProofOptions};
    use crypto::hash::blake3;
    use math::field::{FieldElement, StarkField};

    #[test]
    fn group_assertions() {
        let options = ProofOptions::new(32, 4, 0, blake3);
        let context = ProofContext::new(2, 8, 2, options);

        let groups = super::group_assertions(
            &context,
            &vec![
                Assertion::new(1, 4, FieldElement::new(4)),
                Assertion::new(1, 0, FieldElement::new(2)),
                Assertion::new(0, 0, FieldElement::new(1)),
                Assertion::new(0, 4, FieldElement::new(3)),
                Assertion::new(0, 7, FieldElement::new(5)),
            ],
            &vec![FieldElement::ZERO; 10],
        )
        .unwrap();

        let group1 = &groups[0];
        assert_eq!(group1.constraints[0].register, 0);
        assert_eq!(group1.constraints[0].value, FieldElement::new(1));
        assert_eq!(group1.constraints[1].register, 1);
        assert_eq!(group1.constraints[1].value, FieldElement::new(2));
        // TODO: check divisor

        let group2 = &groups[1];
        assert_eq!(group2.constraints[0].register, 0);
        assert_eq!(group2.constraints[0].value, FieldElement::new(3));
        assert_eq!(group2.constraints[1].register, 1);
        assert_eq!(group2.constraints[1].value, FieldElement::new(4));
        // TODO: check divisor

        let group3 = &groups[2];
        assert_eq!(group3.constraints[0].register, 0);
        assert_eq!(group3.constraints[0].value, FieldElement::new(5));
        // TODO: check divisor
    }
}
