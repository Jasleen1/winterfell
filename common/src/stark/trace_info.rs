#[derive(Copy, Clone)]
pub struct TraceInfo(usize, usize, usize);

impl TraceInfo {
    pub fn new(width: usize, length: usize, blowup: usize) -> Self {
        TraceInfo(width, length, blowup)
    }

    pub fn width(&self) -> usize {
        self.0
    }

    pub fn length(&self) -> usize {
        self.1
    }

    pub fn blowup(&self) -> usize {
        self.2
    }

    pub fn lde_domain_size(&self) -> usize {
        self.length() * self.blowup()
    }
}
