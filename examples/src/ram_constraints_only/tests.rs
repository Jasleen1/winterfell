// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::Blake3_256;
use winterfell::{FieldExtension, ProofOptions};

#[test]
fn ram_constraints_test_basic_proof_verification() {
    let rescue_eg = Box::new(super::RamConstraintsExample::<Blake3_256>::new(
        16,
        64,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification(rescue_eg);
}

#[test]
fn ram_constraints_test_basic_proof_verification_extension() {
    let ram_constraints_eg = Box::new(super::RamConstraintsExample::<Blake3_256>::new(
        8,
        64,
        build_options(true),
    ));
    crate::tests::test_basic_proof_verification(ram_constraints_eg);
}

#[test]
fn ram_constraints_test_basic_proof_verification_fail() {
    let ram_constraints_eg = Box::new(super::RamConstraintsExample::<Blake3_256>::new(
        8,
        64,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification_fail(ram_constraints_eg);
}

fn build_options(use_extension_field: bool) -> ProofOptions {
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(28, 8, 0, extension, 4, 31)
}
