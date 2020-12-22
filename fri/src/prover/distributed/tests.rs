use super::{
    super::tests::{build_evaluations, build_prover_channel, verify_proof},
    FriProver,
};
use crate::{FriOptions, PublicCoin};
use kompact::prelude::*;
use std::io::Write;

#[test]
fn distributed_fri_prove_verify() {
    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    let trace_length = 4096;
    let ce_blowup = 2;
    let lde_blowup = 8;

    let options = FriOptions::new(lde_blowup, crypto::hash::blake3);
    let mut channel = build_prover_channel(trace_length, &options);
    let evaluations = build_evaluations(trace_length, lde_blowup, ce_blowup);

    // instantiate the prover and generate the proof
    let num_workers = 128;
    let system = KompactConfig::default().build().expect("system");
    let mut prover = FriProver::new(&system, options.clone(), num_workers);
    prover.build_layers(&mut channel, &evaluations);
    let positions = channel.draw_query_positions();
    let proof = prover.build_proof(&positions);

    // make sure the proof can be verified
    let commitments = channel.fri_layer_commitments().to_vec();
    let max_degree = trace_length * ce_blowup - 1;
    let result = verify_proof(
        proof,
        commitments,
        &evaluations,
        max_degree,
        &positions,
        &options,
    );
    assert!(result.is_ok(), "{:?}", result);

    system.shutdown().expect("shutdown");
}
