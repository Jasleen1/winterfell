use common::{AssertionConstraintGroup, ConstraintDivisor};
use math::{fft, field::{BaseElement, FieldElement}};

pub fn evaluate_assertions2<E: FieldElement + From<BaseElement>>(
    constraint_groups: &[AssertionConstraintGroup2],
    state: &[E],
    x: E,
    result: &mut Vec<E>,
) {

    let mut degree_adjustment = constraint_groups[0].degree_adjustment;
    let mut xp = x.exp(degree_adjustment.into());

    for group in constraint_groups.iter() {
        if group.degree_adjustment != degree_adjustment {
            degree_adjustment = group.degree_adjustment;
            xp = x.exp(degree_adjustment.into());
        }
        result.push(group.evaluate(state, x, xp));
    }

}

pub fn evaluate_assertions<E: FieldElement + From<BaseElement>>(
    constraint_groups: &[AssertionConstraintGroup],
    state: &[E],
    x: E,
    result: &mut Vec<E>,
) {
    let mut degree_adjustment = constraint_groups[0].degree_adjustment();
    let mut xp = E::exp(x, degree_adjustment.into());

    for group in constraint_groups.iter() {
        if group.degree_adjustment() != degree_adjustment {
            degree_adjustment = group.degree_adjustment();
            xp = E::exp(x, degree_adjustment.into());
        }
        result.push(evaluate_assertion_group(group, state, x, xp));
    }
}

pub fn evaluate_assertion_group<E: FieldElement + From<BaseElement>>(
    group: &AssertionConstraintGroup,
    state: &[E],
    x: E,
    xp: E,
) -> E {
    let mut result = E::ZERO;
    let mut result_adj = E::ZERO;

    for constraint in group.constraints().iter() {
        let evaluation = constraint.evaluate_at(x, state[constraint.register()]);
        result = result + evaluation * E::from(constraint.cc().0);
        result_adj = result_adj + evaluation * E::from(constraint.cc().1);
    }

    result + result_adj * xp
}

// ================================================================================================

pub fn prepare_assertion_constraints(constraint_groups: &[AssertionConstraintGroup]) -> Vec<AssertionConstraintGroup2> {
    let mut result = Vec::with_capacity(constraint_groups.len());
    for group in constraint_groups {
        result.push(AssertionConstraintGroup2::new(group, &[]));
    }
    result
}

struct SingleValueConstraint {
    register: usize,
    value: BaseElement,
    coefficients: (BaseElement, BaseElement)
}

impl SingleValueConstraint {
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E]) -> (E, E) {
        let evaluation = state[self.register] - E::from(self.value);
        (evaluation * E::from(self.coefficients.0), evaluation * E::from(self.coefficients.1))
    }
}

struct PolynomialConstraint {
    register: usize,
    poly: Vec<BaseElement>,
    x_offset: BaseElement,
    coefficients: (BaseElement, BaseElement),
}

impl PolynomialConstraint {
    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], x: E) -> (E, E) {
        let x = x * E::from(self.x_offset);
        let evaluation = state[self.register] - self.poly
            .iter()
            .rev()
            .fold(E::ZERO, |result, coeff| result * x + E::from(*coeff));
        (evaluation * E::from(self.coefficients.0), evaluation * E::from(self.coefficients.1))
    }
}

struct MultiValueConstraint {
    register: usize,
    values: Vec<BaseElement>,
    step_offset: usize,
    coefficients: (BaseElement, BaseElement),
}

pub struct AssertionConstraintGroup2 {
    degree_adjustment: u32,
    divisor: ConstraintDivisor,
    single_value_constraints: Vec<SingleValueConstraint>,
    polynomial_constraints: Vec<PolynomialConstraint>,
    multi_value_cache: Vec<MultiValueConstraint>
}

impl AssertionConstraintGroup2 {
    
    pub fn new(group: &common::AssertionConstraintGroup, twiddles: &[BaseElement]) -> AssertionConstraintGroup2 {

        let mut result = AssertionConstraintGroup2 {
            degree_adjustment: group.degree_adjustment(),
            divisor: group.divisor().clone(),
            single_value_constraints: Vec::new(),
            polynomial_constraints: Vec::new(),
            multi_value_cache: Vec::new(),
        };

        for constraint in group.constraints() {
            if constraint.poly().len() == 1 {
                result.single_value_constraints.push(SingleValueConstraint {
                    register: constraint.register(),
                    value: constraint.poly()[0],
                    coefficients: constraint.cc().clone(),
                });
            }
            else if constraint.poly().len() < 1024 {
                result.polynomial_constraints.push(PolynomialConstraint {
                    register: constraint.register(),
                    poly: constraint.poly().to_vec(),
                    x_offset: constraint.x_offset(),
                    coefficients: constraint.cc().clone(),
                });
            }
            else {
                let mut values = vec![BaseElement::ZERO; twiddles.len() * 2];
                values[..constraint.poly().len()].copy_from_slice(constraint.poly());
                fft::evaluate_poly(&mut values, twiddles, true);

                result.multi_value_cache.push(MultiValueConstraint {
                    register: constraint.register(),
                    values,
                    step_offset: 0,
                    coefficients: constraint.cc().clone(),
                });
            }
        }

        result
    }

    pub fn evaluate<E: FieldElement + From<BaseElement>>(&self, state: &[E], x: E, xp: E) -> E {
        let mut result = E::ZERO;
        let mut result_adj = E::ZERO;
    
        for constraint in self.single_value_constraints.iter() {
            let (ev, ev_adj) = constraint.evaluate(state);
            result = result + ev;
            result_adj = result_adj + ev_adj;
        }

        for constraint in self.polynomial_constraints.iter() {
            let (ev, ev_adj) = constraint.evaluate(state, x);
            result = result + ev;
            result_adj = result_adj + ev_adj;
        }
    
        result + result_adj * xp
    }

    pub fn divisor(&self) -> &ConstraintDivisor {
        &self.divisor
    }
}