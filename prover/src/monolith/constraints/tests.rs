use super::{utils::infer_degree, ConstraintEvaluationTable};
use crate::{
    channel::ProverChannel,
    monolith::{build_trace_tree, extend_trace},
    tests::{build_fib_trace, build_proof_context, FibEvaluator},
};
use common::{
    errors::*, Assertion, ConstraintDivisor, ConstraintEvaluator, DefaultAssertionEvaluator,
    TransitionEvaluator,
};
use crypto::hash::blake3;
use math::field::{BaseElement, FieldElement, FromVec};

#[test]
fn test_fib_evaluate_constraints_good_case() {
    // evaluate constraints
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let assertions = build_fib_assertions(&fib_trace);
    let evaluations = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    )
    .unwrap()
    .into_vec();
    let transition_evaluations = &evaluations[0];
    let input_evaluations = &evaluations[1];
    let output_evaluations = &evaluations[2];

    assert_eq!(ce_blowup_factor, FibEvaluator::get_ce_blowup_factor());

    // transition constraints must be evaluations of degree (trace_length * ce_blowup_factor - 1) polynomial
    assert_eq!(
        trace_length * ce_blowup_factor - 1,
        infer_degree(transition_evaluations)
    );

    // boundary constraints must be evaluations of degree (trace_length + 1) polynomial
    assert_eq!(trace_length + 1, infer_degree(input_evaluations));
    assert_eq!(trace_length + 1, infer_degree(output_evaluations));

    // transition constraint evaluations must be all 0s, except for the last step
    for &evaluation in transition_evaluations
        .iter()
        .rev()
        .skip(ce_blowup_factor)
        .rev()
        .step_by(ce_blowup_factor)
    {
        assert_eq!(BaseElement::ZERO, evaluation);
    }
    assert_ne!(
        BaseElement::ZERO,
        transition_evaluations[(trace_length - 1) * ce_blowup_factor]
    );

    // input assertion evaluations must be 0 only at the first step
    assert_eq!(BaseElement::ZERO, input_evaluations[0]);
    for &evaluation in input_evaluations
        .iter()
        .skip(ce_blowup_factor)
        .step_by(ce_blowup_factor)
    {
        assert_ne!(BaseElement::ZERO, evaluation);
    }

    // output assertion evaluations must be 0 only at the last step
    for &evaluation in output_evaluations
        .iter()
        .rev()
        .skip(ce_blowup_factor)
        .rev()
        .step_by(ce_blowup_factor)
    {
        assert_ne!(BaseElement::ZERO, evaluation);
    }
    assert_eq!(
        BaseElement::ZERO,
        output_evaluations[(trace_length - 1) * ce_blowup_factor]
    );
}

#[test]
fn test_fib_invalid_assertions() {
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    // add an invalid assertion
    let fib_trace_vec = build_fib_trace(trace_length * 2);
    let fib_trace = super::TraceTable::new(fib_trace_vec);
    let mut assertions = build_fib_assertions(&fib_trace);
    assertions[0] = Assertion::new(0, 0, BaseElement::from(2u8));

    let evaluations = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    )
    .unwrap()
    .into_vec();
    let input_evaluations = &evaluations[1];

    // input assertion evaluation will be non-zero
    for &evaluation in input_evaluations.iter() {
        assert_ne!(BaseElement::ZERO, evaluation);
    }
}

#[test]
fn test_bad_fib_evaluate_constraints() {
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    // alter one of the states to be incorrect
    let trace_vec = build_fib_trace(trace_length * 2);
    let mut reg0_extended = trace_vec[0].clone();
    reg0_extended[5] = reg0_extended[5] - BaseElement::from(1u8);
    let trace_vec_extended = vec![reg0_extended, trace_vec[1].clone()];
    let fib_trace_extended = super::TraceTable::new(trace_vec_extended);

    // should throw error
    let assertions = build_fib_assertions(&fib_trace_extended);
    let eval = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace_extended,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    );
    let res = matches!(
        eval,
        Err(ProverError::UnsatisfiedTransitionConstraintError(_))
    );
    assert!(res);
}

#[test]
fn test_fib_duplicate_assertions() {
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    // add a duplicate assertion
    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let mut assertions = build_fib_assertions(&fib_trace);
    assertions.push(Assertion::new(1, 0, BaseElement::from(1u8)));

    // should throw error on duplicate assertion
    let eval = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    );
    let res = matches!(
        eval,
        Err(ProverError::Eval(EvaluatorError::DuplicateAssertion(_, _)))
    );
    assert!(res);
}

#[test]
fn test_fib_assertions_invalid_register() {
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    // add an assertion for an invalid register
    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let mut assertions = build_fib_assertions(&fib_trace);
    assertions.push(Assertion::new(5, 0, BaseElement::from(1u8)));

    // should throw error on invalid register
    let eval = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    );
    let res = matches!(
        eval,
        Err(ProverError::Eval(
            EvaluatorError::InvalidAssertionRegisterIndex(_)
        ))
    );
    assert!(res);
}

#[test]
fn test_fib_assertions_invalid_step() {
    let trace_length = 8; // must be a power of 2
    let lde_blowup_factor = 4; // must be a power of 2
    let ce_blowup_factor = 2; // must be a power of 2, cannot exceed lde_blowup_factor, and cannot exceed 2^(degree of constraints)

    // add a duplicate assertion
    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let mut assertions = build_fib_assertions(&fib_trace);
    assertions.push(Assertion::new(1, 10, BaseElement::from(1u8)));

    // should throw error on invalid step
    let eval = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    );
    let res = matches!(
        eval,
        Err(ProverError::Eval(EvaluatorError::InvalidAssertionStep(_)))
    );
    assert!(res);
}

#[test]
fn build_bad_constraint_poly() {
    // mismatched degrees
    let trace_length = 8;
    let ce_blowup_factor = 2;
    let lde_blowup_factor = 4;
    let context = build_proof_context(trace_length, ce_blowup_factor, lde_blowup_factor);
    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let assertions = build_fib_assertions(&fib_trace);
    let evaluations = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    )
    .unwrap();

    let mut divisors = evaluations.divisors().to_vec();
    let eval_vec = &evaluations.into_vec();

    // Take the first divisor and increase its degree by appending an element to the vec
    let mut exclude = divisors[0].exclude().to_vec();
    exclude.push(BaseElement::from(7u8));
    divisors[0] = ConstraintDivisor::new(divisors[0].numerator().to_vec(), exclude);
    let modified_evaluations = ConstraintEvaluationTable::new(eval_vec.to_vec(), divisors);

    // should throw error if debug assertions is enabled
    let eval = super::build_constraint_poly(modified_evaluations, &context);
    let res = matches!(
        eval,
        Err(ProverError::MismatchedConstraintPolynomialDegree(_, _))
    );
    if cfg!(debug_assertions) {
        assert!(res);
    } else {
        assert!(eval.is_ok());
    }
}

#[test]
fn test_build_constraint_poly() {
    // evaluate constraints
    let trace_length = 8;
    let ce_blowup_factor = 2;
    let lde_blowup_factor = 4;
    let context = build_proof_context(trace_length, ce_blowup_factor, lde_blowup_factor);
    let fib_trace = super::TraceTable::new(build_fib_trace(trace_length * 2));
    let assertions = build_fib_assertions(&fib_trace);
    let evaluations = build_constraint_evaluations::<FibEvaluator, BaseElement>(
        fib_trace,
        ce_blowup_factor,
        lde_blowup_factor,
        assertions,
    )
    .unwrap();

    let constraint_poly = super::build_constraint_poly(evaluations, &context).unwrap();

    assert_eq!(8, constraint_poly.degree());
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_constraint_evaluations<T: TransitionEvaluator, E: FieldElement + FromVec<BaseElement>>(
    trace: super::TraceTable,
    ce_blowup_factor: usize,
    lde_blowup_factor: usize,
    assertions: Vec<Assertion>,
) -> Result<ConstraintEvaluationTable<E>, ProverError> {
    let trace_length = trace.num_states();
    // build proof context
    let context = build_proof_context(trace_length, ce_blowup_factor, lde_blowup_factor);

    let lde_domain = super::super::build_lde_domain(&context);
    let (extended_trace, _) = extend_trace(trace, &lde_domain);

    // commit to the trace
    let mut channel = ProverChannel::new(&context);
    let trace_tree = build_trace_tree(&extended_trace, blake3);
    channel.commit_trace(*trace_tree.root());

    // build constraint evaluator
    let mut evaluator =
        ConstraintEvaluator::<T, DefaultAssertionEvaluator>::new(&channel, &context, assertions)?;

    // evaluate constraints
    super::evaluate_constraints(&mut evaluator, &extended_trace, &lde_domain)
}

fn build_fib_assertions(trace: &super::TraceTable) -> Vec<Assertion> {
    vec![
        Assertion::new(0, 0, BaseElement::from(1u8)),
        Assertion::new(1, 0, BaseElement::from(1u8)),
        Assertion::new(
            1,
            trace.num_states() - 1,
            trace.get(1, trace.num_states() - 1),
        ),
    ]
}
