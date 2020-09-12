use super::trace::TraceTable;
use crate::ProofOptions;
use log::debug;
use math::{fft, field};
use std::time::Instant;

pub struct Prover {
    options: ProofOptions,
}

impl Prover {
    pub fn new(options: ProofOptions) -> Prover {
        Prover { options }
    }

    pub fn prove(&self, trace: Vec<Vec<u128>>) {
        let mut trace = TraceTable::new(trace, &self.options);

        // 1 ----- extend execution trace -------------------------------------------------------------

        // build LDE domain and LDE twiddles (for FFT evaluation over LDE domain)
        let now = Instant::now();
        let (lde_domain, lde_twiddles) = build_lde_domain(trace.domain_size());
        debug!(
            "Built LDE domain of {} elements in {} ms",
            lde_domain.len(),
            now.elapsed().as_millis()
        );

        // extend the trace table; this interpolates each register of the trace into a polynomial,
        // and then evaluates the polynomial over LDE domain
        trace.extend(&lde_twiddles);
        debug!(
            "Extended execution trace of {} registers from {} to {} steps in {} ms",
            trace.register_count(),
            trace.unextended_length(),
            trace.domain_size(),
            now.elapsed().as_millis()
        );

        // 2 ----- Commit to the extended execution trace -----------------------------------------
        let now = Instant::now();
        let _trace_commitment = trace.commit(self.options.hash_fn());
        debug!(
            "Committed to extended execution trace in {} ms",
            now.elapsed().as_millis()
        );

        // 3 ----- Evaluate constraints -----------------------------------------------------------
        // TODO
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_lde_domain(domain_size: usize) -> (Vec<u128>, Vec<u128>) {
    let root = field::get_root_of_unity(domain_size);
    let domain = field::get_power_series(root, domain_size);

    // it is more efficient to build by taking half of the domain and permuting it, rather than
    // building twiddles from scratch using fft::get_twiddles()
    let mut twiddles = domain[..(domain.len() / 2)].to_vec();
    fft::permute(&mut twiddles);

    (domain, twiddles)
}
