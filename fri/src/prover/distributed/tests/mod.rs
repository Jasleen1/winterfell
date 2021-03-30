use super::{
    super::tests::{build_evaluations, build_prover_channel, verify_proof},
    FriProver,
};
use crate::{FriOptions, PublicCoin};
use kompact::prelude::*;
use math::field::{BaseElement, FieldElement};
use std::{net::SocketAddr, time::Duration};

// CONSTANTS
// ================================================================================================

const PROVER_ADDRESS: &str = "127.0.0.1:12345";
const WORKER_ADDRESS: &str = "127.0.0.1:0";

// TESTS
// ================================================================================================

#[test]
fn distributed_fri_prove_verify() {
    let num_workers = 2;
    let trace_length = 512; // TODO: set to larger value - e.g. 4096
    let ce_blowup = 2;
    let lde_blowup = 8;
    let domain_offset = BaseElement::ONE; // TODO: make work with non-unary offset

    let options = FriOptions::new(lde_blowup, domain_offset, crypto::hash::blake3);
    let mut channel = build_prover_channel(trace_length, &options);
    let evaluations = build_evaluations(trace_length, lde_blowup, ce_blowup);

    // start up prover and worker systems
    let prover_socket: SocketAddr = PROVER_ADDRESS.parse().unwrap();
    let prover_system = build_prover_system(prover_socket);
    let mut prover = FriProver::new(&prover_system, options.clone());

    let worker_socket: SocketAddr = WORKER_ADDRESS.parse().unwrap();
    let mut worker_systems: Vec<KompactSystem> = (0..num_workers)
        .map(|_i| run_worker(prover_socket, worker_socket))
        .collect();
    std::thread::sleep(Duration::from_millis(1000));

    // build the proof
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

    // shut down worker and prover systems
    for sys in worker_systems.drain(..) {
        std::thread::sleep(Duration::from_millis(1000));
        sys.shutdown().expect("shutdown");
    }
    std::thread::sleep(Duration::from_millis(1000));
    prover_system.shutdown().expect("shutdown");
}

// STARTUP FUNCTIONS
// ================================================================================================

pub fn build_prover_system(socket: SocketAddr) -> KompactSystem {
    let mut cfg = KompactConfig::new();
    cfg.system_components(DeadletterBox::new, NetworkConfig::new(socket).build());
    cfg.build().expect("KompactSystem")
}

fn run_worker(prover_socket: SocketAddr, worker_socket: SocketAddr) -> KompactSystem {
    let mut cfg = KompactConfig::new();
    cfg.system_components(
        DeadletterBox::new,
        NetworkConfig::new(worker_socket).build(),
    );

    let system = cfg.build().expect("KompactSystem");

    let prover_service: ActorPath = NamedPath::with_socket(
        Transport::TCP,
        prover_socket,
        vec![super::PROVER_PATH.into()],
    )
    .into();

    let (worker, worker_registration) = system
        .create_and_register(|| super::worker::Worker::new(prover_service, crypto::hash::blake3));
    let _path =
        worker_registration.wait_expect(Duration::from_millis(1000), "detector never registered");
    system.start(&worker);

    system
}
