// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use winterfell::{FieldExtension, ProofOptions};

use crate::Blake3_256;

#[test]
fn fft_test_basic_proof_verification() {
    let fft_eg = Box::new(super::FFTRapsExample::<Blake3_256>::new(
        32,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification(fft_eg);
}

#[test]
fn fft_test_basic_proof_verification_extension() {
    let fft_eg = Box::new(super::FFTRapsExample::<Blake3_256>::new(
        32,
        build_options(true),
    ));
    crate::tests::test_basic_proof_verification(fft_eg);
}

#[test]
fn fft_test_basic_proof_verification_fail() {
    let fft_eg = Box::new(super::FFTRapsExample::<Blake3_256>::new(
        32,
        build_options(false),
    ));
    crate::tests::test_basic_proof_verification_fail(fft_eg);
}

fn build_options(use_extension_field: bool) -> ProofOptions {
    let extension = if use_extension_field {
        FieldExtension::Quadratic
    } else {
        FieldExtension::None
    };
    ProofOptions::new(28, 8, 0, extension, 4, 256)
}
