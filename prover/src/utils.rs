#[cfg(test)]
pub fn build_fib_trace(length: usize) -> Vec<Vec<u128>> {
    use math::field::{self, add, mul};

    assert!(length.is_power_of_two(), "length must be a power of 2");

    let mut reg1 = vec![field::ONE];
    let mut reg2 = vec![field::ONE];

    for i in 0..(length / 2 - 1) {
        reg1.push(add(reg1[i], reg2[i]));
        reg2.push(add(reg1[i], mul(2, reg2[i])));
    }

    vec![reg1, reg2]
}
