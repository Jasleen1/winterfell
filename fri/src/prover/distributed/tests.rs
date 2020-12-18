use super::FriProver;
use crate::{DefaultProverChannel, FriOptions};
use kompact::prelude::*;
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

#[test]
fn actor_fri() {
    let trace_length = 4096;
    let ce_blowup = 2;
    let lde_blowup = 8;

    let mut channel = build_prover_channel(trace_length, lde_blowup);
    let evaluations = build_evaluations(trace_length, lde_blowup, ce_blowup);

    let num_workers = 128;
    let system = KompactConfig::default().build().expect("system");
    let mut prover = FriProver::new(&system, num_workers);
    prover.build_layers(&mut channel, &evaluations);

    let positions = channel.draw_query_positions();

    let _proof = prover.build_proof(&positions);

    system.shutdown().expect("shutdown");
    //assert!(false);
}

fn build_prover_channel(trace_length: usize, lde_blowup: usize) -> DefaultProverChannel {
    let options = FriOptions::new(lde_blowup, crypto::hash::blake3);
    DefaultProverChannel::new(options, trace_length * lde_blowup, 32)
}

fn build_evaluations(trace_length: usize, lde_blowup: usize, ce_blowup: usize) -> Vec<BaseElement> {
    let len = (trace_length * ce_blowup) as u128;
    let mut p = (0..len).map(BaseElement::new).collect::<Vec<_>>();
    let domain_size = trace_length * lde_blowup;
    p.resize(domain_size, BaseElement::ZERO);

    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let twiddles = fft::get_twiddles(g, domain_size);

    fft::evaluate_poly(&mut p, &twiddles, true);
    p
}
