use crate::{ComputationContext, ConstraintDivisor};
use crypto::RandomElementGenerator;
use math::field::BaseElement;
use super::Assertions;

// TYPES AND INTERFACES
// ================================================================================================

/// A group of assertion constraints all having the same divisor.
#[derive(Debug, Clone)]
pub struct AssertionConstraintGroup {
    constraints: Vec<AssertionConstraint>,
    divisor: ConstraintDivisor,
    degree_adjustment: u32,
}

#[derive(Debug, Clone)]
pub struct AssertionConstraint {
    pub register: usize,
    pub poly: Vec<BaseElement>,
    pub cc: (BaseElement, BaseElement),
}

// CONSTRAINT BUILDER
// ================================================================================================

pub fn build_assertion_constraints(
    context: &ComputationContext,
    assertions: &Assertions,
    mut coeff_prng: RandomElementGenerator,
) -> Vec<AssertionConstraintGroup> {
    // group assertions by step - i.e.: assertions for the first step are grouped together,
    // assertions for the last step are grouped together etc.
    let mut groups: Vec<AssertionConstraintGroup> = Vec::new();

    // this will iterate over point assertions grouped by step in ascending order
    for (&step, assertions) in assertions.point_assertions() {
        // create a constraint group for each step; the divisor is defined
        // by the step at which the assertion is valid
        let divisor = ConstraintDivisor::from_assertion(&[context.get_trace_x_at(step)]);
        let mut group = AssertionConstraintGroup::new(context, divisor);

        for assertion in assertions {
            // add assertions to the group; these will be sorted by register in ascending order;
            // also we set two composition coefficients per assertion; these will be used to
            // compute random liner combination of constraint evaluations
            group.constraints.push(AssertionConstraint {
                register: assertion.register,
                poly: vec![assertion.value],
                cc: coeff_prng.draw_pair(),
            });
        }

        groups.push(group);
    }

    // TODO: build constraints for cyclic assertions

    // make sure groups are sorted by adjustment degree
    groups.sort_by_key(|c| c.degree_adjustment);

    groups
}

// ASSERTION CONSTRAINT GROUP IMPLEMENTATION
// ================================================================================================

impl AssertionConstraintGroup {
    fn new(context: &ComputationContext, divisor: ConstraintDivisor) -> Self {
        // We want to make sure that once we divide a constraint polynomial by its divisor, the
        // degree of the resulting polynomials will be exactly equal to the composition_degree.
        // Assertion constraint degree is always deg(trace). So, the adjustment degree is simply:
        // deg(composition) + deg(divisor) - deg(trace)
        let target_degree = context.composition_degree() + divisor.degree();
        let degree_adjustment = (target_degree - context.trace_poly_degree()) as u32;

        AssertionConstraintGroup {
            constraints: Vec::new(),
            divisor,
            degree_adjustment,
        }
    }

    pub fn constraints(&self) -> &[AssertionConstraint] {
        &self.constraints
    }

    pub fn divisor(&self) -> &ConstraintDivisor {
        &self.divisor
    }

    pub fn degree_adjustment(&self) -> u32 {
        self.degree_adjustment
    }
}