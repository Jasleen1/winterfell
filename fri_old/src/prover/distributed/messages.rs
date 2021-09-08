use kompact::prelude::*;
use math::field::BaseElement;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// PROVER REQUEST MESSAGES
// ================================================================================================

/// Messages sent from the main thread to the manager.
#[derive(Debug)]
pub enum ProverRequest {
    InitRequest(Ask<RequestInfo, ()>),
    CommitToLayer(Ask<(), Vec<[u8; 32]>>),
    ApplyDrp(Ask<BaseElement, ()>),
    RetrieveRemainder(Ask<(), Vec<BaseElement>>),
    QueryLayers(Ask<Vec<usize>, Vec<Vec<Vec<QueryResult>>>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub index: usize,
    pub value: [BaseElement; 4],
    pub path: Vec<[u8; 32]>,
}

#[derive(Debug)]
pub struct RequestInfo {
    pub evaluations: Arc<Vec<BaseElement>>,
    pub num_partitions: usize,
    pub num_layers: usize,
}

// WORKER CHECK-IN MESSAGE
// ================================================================================================

#[derive(Debug, Clone, Copy)]
pub struct WorkerCheckIn;

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
    const SER_ID: SerId = 101;

    fn deserialise(_buf: &mut dyn Buf) -> Result<WorkerCheckIn, SerError> {
        Ok(WorkerCheckIn)
    }
}

// WORKER REQUEST MESSAGES
// ================================================================================================

// TODO: implement better handling of WorkerRequest serialization/deserialization

/// Messages sent from the manager to the workers.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerRequest {
    AssignPartitions(WorkerPartitions),
    CommitToLayer,
    ApplyDrp(BaseElement),
    RetrieveRemainder,
    Query(Vec<usize>),
    Reset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkerPartitions {
    pub evaluations: Vec<BaseElement>,
    pub num_layers: usize,
    pub num_partitions: usize,
    pub partition_indexes: Vec<usize>,
}

impl Serialisable for WorkerRequest {
    fn ser_id(&self) -> SerId {
        Self::SER_ID
    }

    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }

    fn serialise(&self, buf: &mut dyn BufMut) -> Result<(), SerError> {
        let binary = bincode::serialize(self).unwrap();
        buf.put_u64(binary.len() as u64);
        buf.put_slice(&binary);
        Ok(())
    }

    fn local(self: Box<Self>) -> Result<Box<dyn Any + Send>, Box<dyn Serialisable>> {
        Ok(self)
    }
}

impl Deserialiser<WorkerRequest> for WorkerRequest {
    const SER_ID: SerId = 102;

    fn deserialise(buf: &mut dyn Buf) -> Result<WorkerRequest, SerError> {
        let len = buf.get_u64() as usize;
        let result = bincode::deserialize(&buf.bytes()[..len]).unwrap();
        Ok(result)
    }
}

// WORKER RESPONSE MESSAGES
// ================================================================================================

// TODO: implement better handling of WorkerResponse serialization/deserialization

/// Messages sent from workers to the manager.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerResponse {
    WorkerReady,
    CommitResult(Vec<[u8; 32]>),
    DrpComplete,
    RemainderResult(Vec<BaseElement>),
    QueryResult(Vec<Vec<Vec<QueryResult>>>),
}

impl Serialisable for WorkerResponse {
    fn ser_id(&self) -> SerId {
        Self::SER_ID
    }

    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }

    fn serialise(&self, buf: &mut dyn BufMut) -> Result<(), SerError> {
        let binary = bincode::serialize(self).unwrap();
        buf.put_u64(binary.len() as u64);
        buf.put_slice(&binary);
        Ok(())
    }

    fn local(self: Box<Self>) -> Result<Box<dyn Any + Send>, Box<dyn Serialisable>> {
        Ok(self)
    }
}

impl Deserialiser<WorkerResponse> for WorkerResponse {
    const SER_ID: SerId = 103;

    fn deserialise(buf: &mut dyn Buf) -> Result<WorkerResponse, SerError> {
        let len = buf.get_u64() as usize;
        let result = bincode::deserialize(&buf.bytes()[..len]).unwrap();
        Ok(result)
    }
}
