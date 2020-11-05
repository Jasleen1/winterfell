# Examples
This crate contains examples illustrating how to use Winterfell library.

## Running examples
To run examples of generating and verifying proofs, do the following:

First, compile an optimized version of the `winterfell` binary by running:
```
cargo build --release
```
The binary will be located in `target/release` directory, and you can run it like so:
```
./target/release/winterfell [example name] [example size] [blowup factor] [num queries]
```
All parameters following the example name are optional. The meaning of the parameters is as follows:

* **example name** is the name of the example to run.
* **example size** is is an example-specific measure of computation complexity.
* **blowup factor** is the blowup factor to use during proof generation. The blowup factor must be a power of 2; the default is example-specific.
* **num queries** is the number of queries to include into the proof. The default is example-specific.

Default parameters for each example target proof security of 96-bits. You can adjust them to see how each of the parameters affects proof generation time, proof size, and security level.

Available examples are described below.

### Fibonacci sequence
This is a toy example which generates (and verifies) proofs for computing an n-th term of the [Fibonacci sequence](https://en.wikipedia.org/wiki/Fibonacci_number). You can run it like so:
```
./target/release/winterfell fib [length] [blowup factor] [num queries]
```
where:

* **length** is the term of the Fibonacci sequence to compute. Currently, this must be a power of 2. The default is 1,048,576 (same as 2^20).
* **blowup factor** defaults to 8.
* **num queries** defaults to 32.

### Hash chain
This example generates (and verifies) proofs for computing a hash chain of [Rescue hashes](https://eprint.iacr.org/2019/426). A hash chain is defined as follows:

*H(...H(H(seed))) = result*

where *H* is Rescue hash function.

You can run the example like so:
```
./target/release/winterfell rescue [length] [blowup factor] [num queries]
```
where:

* **length** is length of the hash chain (the number of times the hash function is invoked). Currently, this must be a power of 2. The default is 1024.
* **blowup factor** defaults to 32.
* **num queries** defaults to 32.

### Merkle authentication path
This example generates (and verifies) proofs for verifying a Merkle authentication path. Specifically, given some Merkle tree known to both the prover and the verifier, the prover can prove that they know some value *v*, such that *hash(v)* is a valid tree leaf. This can be used to anonymously prove membership in a Merkle tree.

You can run the example like so:
```
./target/release/winterfell merkle [tree depth] [blowup factor] [num queries]
```
where:

* **tree depth** is the depth of the Merkle tree for which to verify a Merkle authentication path. Currently, the depth must be one less than a power of 2 (e.g. 3, 7, 15). Note that a tree of depth 15 takes about 3 seconds to construct.
* **blowup factor** defaults to 32.
* **num queries** defaults to 32.

### Anonymous token
This example generates (and verifies) proofs for anonymous which are described in detail [here](https://docs.google.com/document/d/1AC5HNB3-d-zqir97r41Bb06vdHhN3M6grKAx-ZNPTHI) (under section 3). At the high level, given `token_seed` and `service_uuid` we define:

* `issued_token` = hash(`token_seed`)
* `subtoken` = hash(`token_seed` | `service_uuid`)

Given a Merkle tree where `issued_token` is a leaf, the example generates a proof that:

* The prover knows pre-image of `issued_token`
* The `issued_token` is a valid leaf in the tree
* That `subtoken` is derived from the right issued_token

The proof does not reveal: the value of `issued_token` or its position in the Merkle tree, or the value of `token_seed`.

You can run the example like so:
```
./target/release/winterfell anon [tree depth] [blowup factor] [num queries]
```
where:

* **tree depth** is the depth of the Merkle tree in which the `issued_token` is stored. Currently, the depth must be one less than a power of 2 (e.g. 3, 7, 15). Note that a tree of depth 15 takes about 3 seconds to construct.
* **blowup factor** defaults to 32.
* **num queries** defaults to 32.