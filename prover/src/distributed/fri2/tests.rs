use super::FriProver;
use crate::{channel::ProverChannel, tests::build_proof_context};
use common::{ComputationContext, PublicCoin};
use kompact::prelude::*;
use math::{
    fft,
    field::{BaseElement, FieldElement, StarkField},
};

#[test]
fn actor_fri() {
    let mut channel = build_prover_channel();
    let evaluations = build_evaluations(channel.context());

    let num_workers = 128;
    let system = KompactConfig::default().build().expect("system");
    let mut prover = FriProver::new(&system, num_workers);
    prover.build_layers(&mut channel, &evaluations);

    channel.grind_query_seed();
    let positions = channel.draw_query_positions();

    let _proof = prover.build_proof(&positions);

    system.shutdown().expect("shutdown");
    assert!(false);
}

fn build_prover_channel() -> ProverChannel {
    let trace_length = 4096;
    let ce_blowup = 2;
    let lde_blowup = 8;
    let context = build_proof_context(trace_length, ce_blowup, lde_blowup);
    ProverChannel::new(&context)
}

fn build_evaluations(ctx: &ComputationContext) -> Vec<BaseElement> {
    let len = (ctx.trace_length() * ctx.ce_blowup_factor()) as u128;
    let mut p = (0..len).map(BaseElement::new).collect::<Vec<_>>();
    let domain_size = ctx.lde_domain_size();
    p.resize(domain_size, BaseElement::ZERO);

    let g = BaseElement::get_root_of_unity(domain_size.trailing_zeros());
    let twiddles = fft::get_twiddles(g, domain_size);

    fft::evaluate_poly(&mut p, &twiddles, true);
    p
}
