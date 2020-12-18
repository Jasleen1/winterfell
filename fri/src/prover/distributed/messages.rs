use kompact::prelude::*;
use math::field::BaseElement;
use std::sync::Arc;

#[derive(Debug)]
pub enum ManagerMessage {
    DistributeEvaluations(Ask<Arc<Vec<BaseElement>>, ()>),
    CommitToLayer(Ask<(), Vec<[u8; 32]>>),
    ApplyDrp(Ask<BaseElement, ()>),
    RetrieveRemainder(Ask<(), Vec<BaseElement>>),
    QueryLayers(Ask<Vec<usize>, Vec<(usize, Vec<Vec<QueryResult>>)>>),
    WorkerReady(usize),
    WorkerCommit(usize, [u8; 32]),
    WorkerDrpComplete(usize),
    WorkerRemainder(usize, BaseElement),
    WorkerQueryResult(usize, Vec<Vec<QueryResult>>),
}

#[derive(Debug)]
pub enum WorkerMessage {
    Prepare(Arc<Vec<BaseElement>>),
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
