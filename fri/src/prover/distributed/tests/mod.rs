use super::{
    super::tests::{build_evaluations, build_prover_channel, verify_proof},
    FriProver,
};
use crate::{FriOptions, PublicCoin};
use kompact::prelude::*;
use std::{io::Write, net::SocketAddr, time::Duration};

// CONSTANTS
// ================================================================================================

const PROVER_ADDRESS: &str = "127.0.0.1:1000";
const WORKER_ADDRESS: &str = "127.0.0.1:0";

const PROVER_PATH: &str = "fri_prover";

// TESTS
// ================================================================================================

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
    let num_workers = 2;
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

#[test]
fn new_distributed_fri_prove_verify() {
    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    let num_workers = 4;
    let trace_length = 4096;
    let ce_blowup = 2;
    let lde_blowup = 8;

    let options = FriOptions::new(lde_blowup, crypto::hash::blake3);
    let mut channel = build_prover_channel(trace_length, &options);
    let evaluations = build_evaluations(trace_length, lde_blowup, ce_blowup);

    // start up prover and verifier systems
    let prover_socket: SocketAddr = PROVER_ADDRESS.parse().unwrap();
    let prover_system = run_prover(prover_socket);
    let worker_socket: SocketAddr = WORKER_ADDRESS.parse().unwrap();
    let mut worker_systems: Vec<KompactSystem> = (0..num_workers)
        .map(|_i| run_worker(prover_socket, worker_socket))
        .collect();
    std::thread::sleep(Duration::from_millis(1000));

    // instantiate the prover
    let mut prover = FriProver::new(&prover_system, options.clone(), num_workers);

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

pub fn run_prover(socket: SocketAddr) -> KompactSystem {
    let mut cfg = KompactConfig::new();
    cfg.system_components(DeadletterBox::new, NetworkConfig::new(socket).build());

    let system = cfg.build().expect("KompactSystem");

    /*
    let (bootstrap, bootstrap_registration) = system.create_and_register(super::Manager::new);
    let bootstrap_service_registration = system.register_by_alias(&bootstrap, PROVER_PATH);

    let _bootstrap_unique = bootstrap_registration
        .wait_expect(Duration::from_millis(1000), "prover never registered");
    let _bootstrap_service = bootstrap_service_registration
        .wait_expect(Duration::from_millis(1000), "prover never registered");
    system.start(&bootstrap);
    */
    system
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
        vec![PROVER_PATH.into()],
    )
    .into();

    /*
    let (detector, registration) =
        system.create_and_register(|| super::worker::Worker::new(bootstrap_service));
    let _path = registration.wait_expect(Duration::from_millis(1000), "detector never registered");
    system.start(&detector);
    */

    system
}