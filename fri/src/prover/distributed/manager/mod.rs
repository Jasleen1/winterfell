use kompact::prelude::*;
use request::RequestState;
use std::collections::HashSet;

use super::messages::{
    ProverRequest, QueryResult, RequestInfo, WorkerCheckIn, WorkerPartitions, WorkerRequest,
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

    // PROVER MESSAGE HANDLERS
    // --------------------------------------------------------------------------------------------

    fn handle_init_request(&mut self, msg: Ask<RequestInfo, ()>) {
        debug!(
            self.ctx.log(),
            "received InitRequest message with {} evaluations",
            msg.request().evaluations.len()
        );
        assert!(
            self.request.is_none(),
            "another request is currently in progress"
        );

        let (promise, request_info) = msg.take();
        let request = RequestState::init(request_info, self.select_workers(), self, promise);
        self.request = Some(request);
    }

    fn handle_commit_to_layer(&mut self, msg: Ask<(), Vec<[u8; 32]>>) {
        debug!(self.ctx.log(), "received CommitToLayer message");
        let (promise, _) = msg.take();

        let mut request = self.request.take().expect("no request in progress");
        request.commit_to_layer(self, promise);
        self.request = Some(request);
    }

    fn handle_apply_drp(&mut self, msg: Ask<BaseElement, ()>) {
        debug!(self.ctx.log(), "received ApplyDrp message");
        let (promise, alpha) = msg.take();

        let mut request = self.request.take().expect("no request in progress");
        request.apply_drp(alpha, self, promise);
        self.request = Some(request);
    }

    fn handle_retrieve_remainder(&mut self, msg: Ask<(), Vec<BaseElement>>) {
        debug!(self.ctx.log(), "received RetrieveRemainder message");
        let (promise, _) = msg.take();

        let mut request = self.request.take().expect("no request in progress");
        request.retrieve_remainder(self, promise);
        self.request = Some(request);
    }

    fn handle_query_layers(&mut self, msg: Ask<Vec<usize>, Vec<Vec<Vec<QueryResult>>>>) {
        debug!(self.ctx.log(), "received QueryLayers message");
        let (promise, positions) = msg.take();

        let mut request = self.request.take().expect("no request in progress");
        request.query_layers(positions, self, promise);
        self.request = Some(request);
    }

    // WORKER MESSAGE HANDLERS
    // --------------------------------------------------------------------------------------------
    fn handle_worker_check_in(&mut self, sender: ActorPath) {
        debug!(self.ctx.log(), "adding worker {} to the pool", sender);
        if !self.workers.insert(sender.clone()) {
            warn!(self.ctx.log(), "worker {} was already registered", sender);
        }
    }

    #[rustfmt::skip]
    fn handle_worker_response(&mut self, worker: ActorPath, response: WorkerResponse) {
        if let Some(request) = &mut self.request {
            match response {
                WorkerResponse::WorkerReady => {
                    debug!(self.ctx.log(), "received WorkerReady message from worker {}", worker);
                    request.handle_worker_ready(worker);
                },
                WorkerResponse::CommitResult(commitments) => {
                    debug!(self.ctx.log(), "received CommitResult message from worker {}", worker);
                    request.handle_worker_commit(worker, commitments);
                },
                WorkerResponse::DrpComplete => {
                    debug!(self.ctx.log(), "received DrpComplete message from worker {}", worker);
                    request.handle_worker_drp_complete(worker);
                },
                WorkerResponse::RemainderResult(remainder) => {
                    debug!(self.ctx.log(), "received RemainderResult message from worker {}", worker);
                    request.handle_worker_remainder(worker, remainder);
                },
                WorkerResponse::QueryResult(results) => {
                    debug!(self.ctx.log(), "received QueryResult message from worker {}", worker);
                    request.handle_worker_query_result(worker, results);
                },
            }
        }
        else {
            panic!("received response from worker {}, but no request is currently in progress", worker);
        }
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------
    fn select_workers(&self) -> Vec<ActorPath> {
        let mut num_workers = self.workers.len();
        if !num_workers.is_power_of_two() {
            num_workers = num_workers.next_power_of_two() >> 1;
        }
        self.workers.iter().take(num_workers).cloned().collect()
    }
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl Actor for Manager {
    type Message = ProverRequest;

    /// Handles messages received from the main thread.
    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg {
            ProverRequest::InitRequest(msg) => self.handle_init_request(msg),
            ProverRequest::CommitToLayer(msg) => self.handle_commit_to_layer(msg),
            ProverRequest::ApplyDrp(msg) => self.handle_apply_drp(msg),
            ProverRequest::RetrieveRemainder(msg) => self.handle_retrieve_remainder(msg),
            ProverRequest::QueryLayers(msg) => self.handle_query_layers(msg),
        }
        Handled::Ok
    }

    /// Handles messages from workers. These messages can fall into two groups:
    /// 1. Check-in messages sent by workers when they come online;
    /// 2. Messages sent by workers in response to a request sent from the prover.
    fn receive_network(&mut self, msg: NetMessage) -> Handled {
        let sender = msg.sender;
        match_deser!(msg.data; {
            checkin: WorkerCheckIn [WorkerCheckIn] => self.handle_worker_check_in(sender),
            response: WorkerResponse [WorkerResponse] => self.handle_worker_response(sender, response),
        });

        Handled::Ok
    }
}

impl ComponentLifecycle for Manager {}
