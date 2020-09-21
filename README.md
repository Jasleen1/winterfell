# Winterfell
This is an experimental project for building a distributed STARK prover.

## Running examples
To run examples of generating and verifying proofs, do the following:

First, compile an optimized version of the `winterfell` binary by running:
```
cargo build --release
```
The binary will be located in `target/release` directory, and you can run it like so:
```
./target/release/winterfell [example name]
```
Currently, the only available example is generating a proof of computing a Fibonacci sequence, and you can run it like so:
```
./target/release/winterfell fib [length] [blowup factor] [num queries]
```
All parameters following the example name are optional. The meaning of the parameters is as follows:

* **length** - term of the Fibonacci sequence to compute. Currently, this must be a power of 2. The default is 1,048,576 (same as 2^20).
* **blowup factor** - blowup up factor to use during proof generation. This must be a power of 2. The default is 8.
* **num queries** - number of queries to include into the proof. The default is 32.

The default settings should generate a proof in about 6 seconds, and security level of the proof will be 96-bit. You can adjust the above parameter to see how each of them affect proof generation time, proof size, and security level.


## References
A STARK is a novel proof-of-computation scheme that allows you to create an efficiently verifiable proof that a computation was executed correctly. The scheme was developed by Eli-Ben Sasson and team at Technion - Israel Institute of Technology. STARKs do not require an initial trusted setup, and rely on very few cryptographic assumptions.

Here are some resources to learn more about STARKs:

* STARKs whitepaper: [Scalable, transparent, and post-quantum secure computational integrity](https://eprint.iacr.org/2018/046)
* STARKs vs. SNARKs: [A Cambrian Explosion of Crypto Proofs](https://nakamoto.com/cambrian-explosion-of-crypto-proofs/)

Vitalik Buterin's blog series on zk-STARKs:
* [STARKs, part 1: Proofs with Polynomials](https://vitalik.ca/general/2017/11/09/starks_part_1.html)
* [STARKs, part 2: Thank Goodness it's FRI-day](https://vitalik.ca/general/2017/11/22/starks_part_2.html)
* [STARKs, part 3: Into the Weeds](https://vitalik.ca/general/2018/07/21/starks_part_3.html)

StarkWare's STARK Math blog series:
* [STARK Math: The Journey Begins](https://medium.com/starkware/stark-math-the-journey-begins-51bd2b063c71)
* [Arithmetization I](https://medium.com/starkware/arithmetization-i-15c046390862)
* [Arithmetization II](https://medium.com/starkware/arithmetization-ii-403c3b3f4355)
* [Low Degree Testing](https://medium.com/starkware/low-degree-testing-f7614f5172db)
* [A Framework for Efficient STARKs](https://medium.com/starkware/a-framework-for-efficient-starks-19608ba06fbe)
