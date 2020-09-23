#[derive(Copy, Clone)]
pub struct TraceInfo {
    width: usize,
    length: usize,
    blowup: usize,
}

impl TraceInfo {
    pub fn new(width: usize, length: usize, blowup: usize) -> Self {
        TraceInfo {
            width,
            length,
            blowup,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn blowup(&self) -> usize {
        self.blowup
    }

    pub fn lde_domain_size(&self) -> usize {
        self.length() * self.blowup()
    }
}
