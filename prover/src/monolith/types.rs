// TRACE TABLE
// ================================================================================================
pub struct TraceTable(Vec<Vec<u128>>);

impl TraceTable {
    pub fn new(registers: Vec<Vec<u128>>) -> TraceTable {
        assert!(
            registers.len() > 0,
            "execution trace must consist of at least one register"
        );
        let trace_length = registers[0].len();
        assert!(
            trace_length.is_power_of_two(),
            "execution trace length must be a power of 2"
        );
        for register in registers.iter() {
            assert!(
                register.len() == trace_length,
                "all register traces must have the same length"
            );
        }

        TraceTable(registers)
    }

    pub fn num_states(&self) -> usize {
        self.0[0].len()
    }

    pub fn num_registers(&self) -> usize {
        self.0.len()
    }

    pub fn copy_row(&self, idx: usize, destination: &mut [u128]) {
        for (i, register) in self.0.iter().enumerate() {
            destination[i] = register[idx];
        }
    }

    #[cfg(test)]
    pub fn get(&self, register: usize, step: usize) -> u128 {
        self.0[register][step]
    }

    #[cfg(test)]
    pub fn get_register(&self, idx: usize) -> &[u128] {
        &self.0[idx]
    }

    pub fn into_vec(self) -> Vec<Vec<u128>> {
        self.0
    }
}

// POLYNOMIAL TABLE
// ================================================================================================
pub struct PolyTable(Vec<Vec<u128>>);

impl PolyTable {
    pub fn new(polys: Vec<Vec<u128>>) -> PolyTable {
        assert!(
            polys.len() > 0,
            "polynomial table must contain at least one polynomial"
        );
        let poly_size = polys[0].len();
        assert!(
            poly_size.is_power_of_two(),
            "polynomial size must be a power of 2"
        );
        for poly in polys.iter() {
            assert!(
                poly.len() == poly_size,
                "all polynomials must have the same size"
            );
        }

        PolyTable(polys)
    }

    pub fn poly_size(&self) -> usize {
        self.0[0].len()
    }

    #[cfg(test)]
    pub fn num_polys(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub fn get_poly(&self, idx: usize) -> &[u128] {
        &self.0[idx]
    }
}
