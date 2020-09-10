# Math
This crate contains modules with mathematical operations needed in STARK proof generation.

## Finite field
[Finite field](src/field) module implements arithmetic operations in a 128-bit prime field such as:

* Basic arithmetic operations: addition, multiplication, subtraction, division, inversion.
* Drawing random and pseudo-random elements from the field.
* Computing roots of unity.

The modulus of the filed is currently set to 2<sup>128</sup> - 45 * 2<sup>40</sup> + 1. This means that the the largest multiplicative subgroup has size of 2<sup>40</sup>.

## Polynomials
[Polynomials](src/polynom) module implements basic polynomial operations such as:

* Evaluation of a polynomial at a single point.
* Interpolation of a polynomial from a set of points (using Lagrange interpolation).
* Addition, multiplication, subtraction, and division of polynomials.
* Synthetic polynomial division.

## Fast Fourier transform
[FFT](src/fft) module contains operations for computing Fast Fourier transform in a prime field. This can be used to interpolate and evaluate polynomials in *O(n log n)* time as long as the domain of the polynomial is multiplicative subgroup with a size which is a power of 2.
