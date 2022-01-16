// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use math::fields::f128::BaseElement;

use crate::arith_parser_example::reading_arith;

pub(crate) fn r1cs_end_to_end_example(input_file: &str) {
    let _r1cs = reading_arith::<BaseElement>(input_file, false);
    // TODO
}
