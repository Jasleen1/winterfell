// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::Blake3_256;
use winterfell::{FieldExtension, ProofOptions};

#[test]
fn pointer_chase_test_basic_proof_verification() {
    let pointer_chase_eg = Box::new(super::PointerChasingCompExample::<Blake3_256>::new(
        16,
        128,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification(pointer_chase_eg);
}

#[test]
fn pointer_chase_test_basic_proof_verification_extension() {
    let pointer_chase_eg = Box::new(super::PointerChasingCompExample::<Blake3_256>::new(
        16,
        128,
        build_options(true),
    ));
    crate::tests::test_basic_proof_verification(pointer_chase_eg);
}

#[test]
fn pointer_chase_test_basic_proof_verification_fail() {
    let pointer_chase_eg = Box::new(super::PointerChasingCompExample::<Blake3_256>::new(
        16,
        128,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification_fail(pointer_chase_eg);
}

fn build_options(use_extension_field: bool) -> ProofOptions {
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(28, 8, 0, extension, 4, 31)
}
