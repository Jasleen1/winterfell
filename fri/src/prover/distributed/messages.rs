use kompact::prelude::*;
use math::field::BaseElement;
use std::sync::Arc;

#[derive(Debug)]
pub enum ManagerMessage {
    ProverRequest(ProverRequest),
    WorkerResponse(WorkerResponse),
}

/// Messages sent from the main thread to the manager.
#[derive(Debug)]
pub enum ProverRequest {
    DistributeEvaluations(Ask<Evaluations, ()>),
    CommitToLayer(Ask<(), Vec<[u8; 32]>>),
    ApplyDrp(Ask<BaseElement, ()>),
    RetrieveRemainder(Ask<(), Vec<BaseElement>>),
    QueryLayers(Ask<Vec<usize>, Vec<(usize, Vec<Vec<QueryResult>>)>>),
}

/// Messages sent from workers to the manager.
#[derive(Debug)]
pub enum WorkerResponse {
    WorkerReady(usize),
    CommitResult(usize, Vec<[u8; 32]>),
    DrpComplete(usize),
    RemainderResult(usize, Vec<BaseElement>),
    QueryResult(usize, Vec<Vec<Vec<QueryResult>>>),
}

/// Messages sent from the manager to the workers.
#[derive(Debug)]
pub enum WorkerRequest {
    Prepare(WorkerPartitions),
    CommitToLayer,
    ApplyDrp(BaseElement),
    RetrieveRemainder,
    Query(Vec<usize>),
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub index: usize,
    pub value: [BaseElement; 4],
    pub path: Vec<[u8; 32]>,
}

#[derive(Debug)]
pub struct Evaluations {
    pub evaluations: Arc<Vec<BaseElement>>,
    pub num_partitions: usize,
}

#[derive(Debug)]
pub struct WorkerPartitions {
    pub evaluations: Arc<Vec<BaseElement>>,
    pub num_partitions: usize,
    pub partition_indexes: Vec<usize>,
}

// WORKER CHECK-IN MESSAGE
// ================================================================================================

#[derive(Debug, Clone, Copy)]
struct WorkerCheckIn;

impl Serialisable for WorkerCheckIn {
    fn ser_id(&self) -> SerId {
        Self::SER_ID
    }

    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }

    fn serialise(&self, _buf: &mut dyn BufMut) -> Result<(), SerError> {
        Ok(())
    }

    fn local(self: Box<Self>) -> Result<Box<dyn Any + Send>, Box<dyn Serialisable>> {
        Ok(self)
    }
}

impl Deserialiser<WorkerCheckIn> for WorkerCheckIn {
    const SER_ID: SerId = 3456;

    fn deserialise(_buf: &mut dyn Buf) -> Result<WorkerCheckIn, SerError> {
        Ok(WorkerCheckIn)
    }
}
