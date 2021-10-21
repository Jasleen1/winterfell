pub trait PolyCommit {
    type Channel;
    type Options;
    type Queries;
    pub fn new(options: Self::Options) -> Self;
    // pub fn commit(&mut self, poly: Polynomial, channel: &mut Self::Channel, domain: Vec<Self::Domain>) -> ;
    // pub fn commit(&mut self, poly: Polynomial, channel: &mut Self::Channel);
    // pub fn prove(&mut self, channel: &mut Self::Channel, values: Vec<Self::Queries>, )
}
