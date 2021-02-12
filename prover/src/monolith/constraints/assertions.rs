use common::AssertionConstraintGroup;
use math::{
    field::{BaseElement, FieldElement},
    polynom,
};

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
        let value = if constraint.poly().len() == 1 {
            E::from(constraint.poly()[0])
        } else {
            let poly: Vec<E> = constraint.poly().iter().map(|&c| E::from(c)).collect();
            polynom::eval(&poly, x)
        };
        let value = state[constraint.register()] - value;
        result = result + value * E::from(constraint.cc().0);
        result_adj = result_adj + value * E::from(constraint.cc().1);
    }

    result + result_adj * xp
}
