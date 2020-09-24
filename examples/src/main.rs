use log::debug;
use std::time::Instant;
use std::{env, io::Write};

use winterfell::fibonacci;

// EXAMPLE RUNNER
// ================================================================================================

fn main() {
    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    // determine the example to run based on command-line arguments
    let args: Vec<String> = env::args().collect();
    let (example, n, blowup_factor, num_queries) = parse_args(args);
    let example = match example.as_str() {
        "fib" => fibonacci::get_example(),
        _ => panic!("example name '{}' is not valid", example),
    };

    debug!("============================================================");
    // generate proof
    let now = Instant::now();
    let (proof, assertions) = example.prove(n, blowup_factor, num_queries);
    debug!(
        "---------------------\nProof generated in {} ms",
        now.elapsed().as_millis()
    );
    let proof_bytes = bincode::serialize(&proof).unwrap();
    debug!("Proof size: {} KB", proof_bytes.len() / 1024);
    debug!("Proof security: {} bits", proof.security_level(true));

    // verify the proof
    debug!("---------------------");
    let now = Instant::now();
    match example.verify(proof, assertions) {
        Ok(_) => debug!("Proof verified in {} ms", now.elapsed().as_millis()),
        Err(msg) => debug!("Failed to verify proof: {}", msg),
    }
    debug!("============================================================");
}

// HELPER FUNCTIONS
// ================================================================================================

fn parse_args(args: Vec<String>) -> (String, usize, usize, usize) {
    if args.len() < 2 {
        ("fib".to_string(), 0, 0, 0)
    } else if args.len() < 3 {
        (args[1].to_string(), 0, 0, 0)
    } else if args.len() < 4 {
        let n = args[2].parse().unwrap();
        (args[1].to_string(), n, 0, 0)
    } else if args.len() < 5 {
        let n = args[2].parse().unwrap();
        let blowup_factor = args[3].parse().unwrap();
        (args[1].to_string(), n, blowup_factor, 0)
    } else {
        let n = args[2].parse().unwrap();
        let blowup_factor = args[3].parse().unwrap();
        let num_queries = args[4].parse().unwrap();
        (args[1].to_string(), n, blowup_factor, num_queries)
    }
}
