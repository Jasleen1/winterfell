// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! This crate contains models and representations for zero knowledge proofs.

pub mod arith_parser;
pub mod r1cs;

mod errors;
pub use errors::{MerkleTreeError, RandomCoinError};