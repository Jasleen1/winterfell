use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crypto::lamport::{
    LamportPlusExtendedPrivateKey, LamportPlusExtendedPublicKey, LamportPlusFinalPublicKey,
};
use sha2::Sha256;
use sha3::Sha3_256;

pub fn lamportplus_blake3_verify(c: &mut Criterion) {
    use blake3::Hasher as Blake3;

    let seed = [0u8; 32];
    let priv_key = LamportPlusExtendedPrivateKey::<Blake3>::generate(&seed);
    let pub_key: LamportPlusExtendedPublicKey<Blake3> = priv_key.clone().into();
    let final_pub_key: LamportPlusFinalPublicKey<Blake3> = pub_key.into();
    let message = "Hello World".as_bytes();
    let sig = priv_key.sign(message);

    c.bench_function("lamportplus_blake3_verify", |bench| {
        bench.iter(|| sig.verify(black_box(&message), black_box(&final_pub_key)))
    });
}

pub fn lamportplus_sha3_256_verify(c: &mut Criterion) {
    let seed = [0u8; 32];
    let priv_key = LamportPlusExtendedPrivateKey::<Sha3_256>::generate(&seed);
    let pub_key: LamportPlusExtendedPublicKey<Sha3_256> = priv_key.clone().into();
    let final_pub_key: LamportPlusFinalPublicKey<Sha3_256> = pub_key.into();
    let message = "Hello World".as_bytes();
    let sig = priv_key.sign(message);

    c.bench_function("lamportplus_sha3_256_verify", |bench| {
        bench.iter(|| sig.verify(black_box(&message), black_box(&final_pub_key)))
    });
}

pub fn lamportplus_sha2_256_verify(c: &mut Criterion) {
    let seed = [0u8; 32];
    let priv_key = LamportPlusExtendedPrivateKey::<Sha256>::generate(&seed);
    let pub_key: LamportPlusExtendedPublicKey<Sha256> = priv_key.clone().into();
    let final_pub_key: LamportPlusFinalPublicKey<Sha256> = pub_key.into();
    let message = "Hello World".as_bytes();
    let sig = priv_key.sign(message);

    c.bench_function("lamportplus_sha2_256_verify", |bench| {
        bench.iter(|| sig.verify(black_box(&message), black_box(&final_pub_key)))
    });
}

criterion_group!(
    lamport_group,
    lamportplus_blake3_verify,
    lamportplus_sha3_256_verify,
    lamportplus_sha2_256_verify
);
criterion_main!(lamport_group);
