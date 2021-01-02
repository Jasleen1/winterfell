use kompact::prelude::*;
use log::{debug, warn};
use request::RequestState;
use std::collections::HashSet;
use std::sync::Arc;

use super::messages::{
    Evaluations, ProverRequest, QueryResult, WorkerCheckIn, WorkerPartitions, WorkerRequest,
    WorkerResponse,
};
use math::field::BaseElement;

mod request;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Manager {
    ctx: ComponentContext<Self>,
    workers: HashSet<ActorPath>,
    request: Option<request::RequestState>,
}

// MANAGER IMPLEMENTATION
// ================================================================================================

impl Manager {
    pub fn new() -> Self {
        Manager {
            ctx: ComponentContext::uninitialised(),
            workers: HashSet::new(),
            request: None,
        }
    }

    // CHECK-IN HANDLER
    // --------------------------------------------------------------------------------------------
    fn handle_worker_check_in(&mut self, sender: ActorPath) {
        debug!("manager: adding worker {} to the pool", sender);
        if !self.workers.insert(sender.clone()) {
            warn!("worker {} was already registered", sender);
        }
    }

    // DISTRIBUTE EVALUATIONS WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_distribute_evaluations(&mut self, msg: Ask<Evaluations, ()>) {
        debug!(
            "manager: received DistributeEvaluations message with {} evaluations",
            msg.request().evaluations.len()
        );
        let (
            promise,
            Evaluations {
                evaluations,
                num_partitions,
            },
        ) = msg.take();
        let mut request = RequestState::new(self.workers.iter().cloned().collect(), num_partitions);
        request.distribute_evaluation(evaluations, self, promise);
        self.request = Some(request);
    }

    fn handle_worker_ready(&mut self, sender: ActorPath) {
        debug!(
            "manager: received WorkerReady message from worker {}",
            sender
        );
        match &mut self.request {
            Some(r) => r.handle_worker_ready(sender),
            None => panic!("no request"),
        }
    }

    // COMMIT TO LAYER WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_commit_to_layer(&mut self, msg: Ask<(), Vec<[u8; 32]>>) {
        debug!("manager: received CommitToLayer message");
        let (promise, _) = msg.take();

        let mut request = self.request.take().unwrap();
        request.commit_to_layer(self, promise);
        self.request = Some(request);
    }

    fn handle_worker_commit(&mut self, sender: ActorPath, worker_commitments: Vec<[u8; 32]>) {
        debug!(
            "manager: received WorkerCommit message from worker {}",
            sender
        );
        match &mut self.request {
            Some(r) => r.handle_worker_commit(sender, worker_commitments),
            None => panic!("no request"),
        }
    }

    // APPLY DRP WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_apply_drp(&mut self, msg: Ask<BaseElement, ()>) {
        debug!("manager: received ApplyDrp message");
        let (promise, alpha) = msg.take();

        let mut request = self.request.take().unwrap();
        request.apply_drp(alpha, self, promise);
        self.request = Some(request);
    }

    fn handle_worker_drp_complete(&mut self, sender: ActorPath) {
        debug!(
            "manager: received WorkerDrpComplete message from worker {}",
            sender
        );
        match &mut self.request {
            Some(r) => r.handle_worker_drp_complete(sender),
            None => panic!("no request"),
        }
    }

    // RETRIEVE REMAINDER WORKFLOW
    // --------------------------------------------------------------------------------------------
    fn handle_retrieve_remainder(&mut self, msg: Ask<(), Vec<BaseElement>>) {
        debug!("manager: received RetrieveRemainder message");
        let (promise, _) = msg.take();

        let mut request = self.request.take().unwrap();
        request.retrieve_remainder(self, promise);
        self.request = Some(request);
    }

    fn handle_worker_remainder(&mut self, sender: ActorPath, worker_remainder: Vec<BaseElement>) {
        debug!(
            "manager: received WorkerRemainder message from worker {}",
            sender
        );
        match &mut self.request {
            Some(r) => r.handle_worker_remainder(sender, worker_remainder),
            None => panic!("no request"),
        }
    }

    // QUERY LAYERS WORKFLOW
    // --------------------------------------------------------------------------------------------
    fn handle_query_layers(&mut self, msg: Ask<Vec<usize>, Vec<Vec<Vec<QueryResult>>>>) {
        debug!("manager: received QueryLayers message");
        let (promise, positions) = msg.take();

        let mut request = self.request.take().unwrap();
        request.query_layers(positions, self, promise);
        self.request = Some(request);
    }

    fn handle_worker_query_result(
        &mut self,
        sender: ActorPath,
        worker_results: Vec<Vec<Vec<QueryResult>>>,
    ) {
        debug!(
            "manager: received WorkerQueryResult message from worker {}",
            sender
        );
        match &mut self.request {
            Some(r) => r.handle_worker_query_result(sender, worker_results),
            None => panic!("no request"),
        }
    }
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl Actor for Manager {
    type Message = ProverRequest;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg {
            ProverRequest::DistributeEvaluations(msg) => self.handle_distribute_evaluations(msg),
            ProverRequest::CommitToLayer(msg) => self.handle_commit_to_layer(msg),
            ProverRequest::ApplyDrp(msg) => self.handle_apply_drp(msg),
            ProverRequest::RetrieveRemainder(msg) => self.handle_retrieve_remainder(msg),
            ProverRequest::QueryLayers(msg) => self.handle_query_layers(msg),
        }

        /*
        if self.request.is_handled() {
            self.request = ManagerRequest::None;
        }
        */

        Handled::Ok
    }

    fn receive_network(&mut self, msg: NetMessage) -> Handled {
        let sender = msg.sender;
        match_deser!(msg.data; {
            checkin: WorkerCheckIn [WorkerCheckIn] => self.handle_worker_check_in(sender),
            response: WorkerResponse [WorkerResponse] => {
                match response {
                    WorkerResponse::WorkerReady => self.handle_worker_ready(sender),
                    WorkerResponse::CommitResult(commitments) => self.handle_worker_commit(sender, commitments),
                    WorkerResponse::DrpComplete => self.handle_worker_drp_complete(sender),
                    WorkerResponse::RemainderResult(remainder) => self.handle_worker_remainder(sender, remainder),
                    WorkerResponse::QueryResult(results) => self.handle_worker_query_result(sender, results),
                    _ => panic!("not implemented")
                }
            },
        });

        Handled::Ok
    }
}

impl ComponentLifecycle for Manager {}

// HELPER FUNCTIONS
// ================================================================================================
fn build_worker_partitions(
    worker_idx: usize,
    num_workers: usize,
    num_partitions: usize,
    evaluations: Arc<Vec<BaseElement>>,
) -> WorkerPartitions {
    let partitions_per_worker = num_partitions / num_workers;
    let partition_indexes = (0..num_partitions)
        .skip(worker_idx * partitions_per_worker)
        .take(partitions_per_worker)
        .collect();

    WorkerPartitions {
        evaluations: evaluations.to_vec(),
        num_partitions,
        partition_indexes,
    }
}
