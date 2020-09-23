# STARK prover
This crate contains implementations of STARK provers all exposing the same public interface.

## Provers
The following provers are (or will be) implemented:

* [Monolith](src/monolith) is prover which runs on a single machine. Currently this prover is single-threaded, but support for multi-threading will be added in the future.
* [Distributed](src/distributed) is a prove which is intended to distribute proof generation over multiple machines. This prover is not yet implemented.

## Interfaces
Proof generation is done via `Prover.prover()` method. To invoke this method, you first need instantiate a prover like so:
```Rust
let prover = Prover::<TransitionEvaluator, AssertionEvaluator>::new(options);
```
where:

* `TransitionEvaluator` describes how transition constraints for the computation are to be evaluated.
* `AssertionEvaluator` describes how assertion constraints for the computation are to be evaluated.
* `options` defines basic properties for proof generation such as: number of queries, blowup factor, grinding factor, and hash function to be used during proof generation. These properties directly inform such metrics as proof generation time, proof size, and proof security level.

Once the prover is instantiated, you can use it to generate proofs like so:
```Rust
let proof = prover.prove(execution_trace, assertions);
```
where:

* `execution_trace` is a two-dimensional matrix describing the trace resulting fro executing the computation against specific inputs (see more [here](#Execution-trace)).
* `assertions` is a list of assertions which must hold against the execution trace for the computation to be valid (see more [here](#Assertions)).

### Execution trace
TODO

### Assertions
TODO