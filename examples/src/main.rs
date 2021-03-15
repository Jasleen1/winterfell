use log::debug;
use std::io::Write;
use std::time::Instant;
use structopt::StructOpt;
use winterfell::{anon, fibonacci, lamport, merkle, rescue, ExampleOptions, ExampleType};

// EXAMPLE RUNNER
// ================================================================================================

fn main() {
    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    // read command-line args
    let options = ExampleOptions::from_args();

    debug!("============================================================");

    // instantiate and prepare the example
    let (example, assertions) = match options.example {
        ExampleType::Fib { sequence_length } => {
            let mut e = fibonacci::fib2::get_example(options);
            let a = e.prepare(sequence_length);
            (e, a)
        }
        ExampleType::Fib8 { sequence_length } => {
            let mut e = fibonacci::fib8::get_example(options);
            let a = e.prepare(sequence_length);
            (e, a)
        }
        ExampleType::Mulfib { sequence_length } => {
            let mut e = fibonacci::mulfib2::get_example(options);
            let a = e.prepare(sequence_length);
            (e, a)
        }
        ExampleType::Mulfib8 { sequence_length } => {
            let mut e = fibonacci::mulfib8::get_example(options);
            let a = e.prepare(sequence_length);
            (e, a)
        }
        ExampleType::Rescue { chain_length } => {
            let mut e = rescue::get_example(options);
            let a = e.prepare(chain_length);
            (e, a)
        }
        ExampleType::Merkle { tree_depth } => {
            let mut e = merkle::get_example(options);
            let a = e.prepare(tree_depth);
            (e, a)
        }
        ExampleType::Anon { tree_depth } => {
            let mut e = anon::get_example(options);
            let a = e.prepare(tree_depth);
            (e, a)
        }
        ExampleType::LamportA { num_signatures } => {
            let mut e = lamport::aggregate::get_example(options);
            let a = e.prepare(num_signatures);
            (e, a)
        }
        ExampleType::LamportT { num_signers } => {
            let mut e = lamport::threshold::get_example(options);
            let a = e.prepare(num_signers);
            (e, a)
        }
    };

    // generate proof
    let now = Instant::now();
    let proof = example.prove(assertions.clone());
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
