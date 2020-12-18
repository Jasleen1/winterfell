use kompact::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use super::{
    messages::{ManagerMessage, QueryResult, WorkerMessage},
    worker::{Worker, WorkerConfig},
};
use math::field::BaseElement;

// TYPES AND INTERFACES
// ================================================================================================

#[derive(ComponentDefinition)]
pub struct Manager {
    ctx: ComponentContext<Self>,
    workers: Vec<Arc<Component<Worker>>>,
    worker_refs: Vec<ActorRefStrong<WithSender<WorkerMessage, ManagerMessage>>>,
    num_workers: usize,
    request: ManagerRequest,
}

// MANAGER IMPLEMENTATION
// ================================================================================================

impl Manager {
    pub fn new(num_workers: usize) -> Self {
        Manager {
            ctx: ComponentContext::uninitialised(),
            workers: Vec::with_capacity(num_workers),
            worker_refs: Vec::with_capacity(num_workers),
            num_workers,
            request: ManagerRequest::None,
        }
    }

    // DISTRIBUTE EVALUATIONS WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_distribute_evaluations(&mut self, msg: Ask<Arc<Vec<BaseElement>>, ()>) {
        println!(
            "manager: received DistributeEvaluations message with {} evaluations",
            msg.request().len()
        );
        match self.request {
            ManagerRequest::None => {
                let evaluations = msg.request().clone();
                self.request = ManagerRequest::DistributeEvaluations {
                    request: Some(msg),
                    worker_replies: HashSet::new(),
                };
                for worker in self.worker_refs.iter() {
                    let msg = WorkerMessage::Prepare(evaluations.clone());
                    worker.tell(WithSender::from(msg, self));
                }
            }
            _ => panic!("an outstanding request is already in progress"),
        }
    }

    fn handle_worker_ready(&mut self, worker_idx: usize) {
        println!(
            "manager: received WorkerReady message from worker {}",
            worker_idx
        );
        match &mut self.request {
            ManagerRequest::DistributeEvaluations {
                request,
                worker_replies,
            } => {
                worker_replies.insert(worker_idx);
                if worker_replies.len() == self.num_workers {
                    let request = request.take().expect("request");
                    request.reply(()).unwrap();
                }
            }
            _ => panic!("DistributeEvaluations request is not in progress"),
        }
    }

    // COMMIT TO LAYER WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_commit_to_layer(&mut self, msg: Ask<(), Vec<[u8; 32]>>) {
        println!("manager: received CommitToLayer message");
        match self.request {
            ManagerRequest::None => {
                self.request = ManagerRequest::CommitToLayer {
                    request: Some(msg),
                    worker_commitments: BTreeMap::new(),
                };
                for worker in self.worker_refs.iter() {
                    worker.tell(WithSender::from(WorkerMessage::CommitToLayer, self));
                }
            }
            _ => panic!("an outstanding request is already in progress"),
        }
    }

    fn handle_worker_commit(&mut self, worker_idx: usize, commitment: [u8; 32]) {
        println!(
            "manager: received WorkerCommit message from worker {}",
            worker_idx
        );
        match &mut self.request {
            ManagerRequest::CommitToLayer {
                request,
                worker_commitments,
            } => {
                worker_commitments.insert(worker_idx, commitment);
                if worker_commitments.len() == self.num_workers {
                    let request = request.take().expect("request");
                    request
                        .reply(worker_commitments.values().cloned().collect())
                        .unwrap();
                }
            }
            _ => panic!("CommitToLayer request is not in progress"),
        }
    }

    // APPLY DRP WORKFLOW
    // --------------------------------------------------------------------------------------------

    fn handle_apply_drp(&mut self, msg: Ask<BaseElement, ()>) {
        println!("manager: received ApplyDrp message");
        match self.request {
            ManagerRequest::None => {
                let alpha = *msg.request();
                self.request = ManagerRequest::ApplyDrp {
                    request: Some(msg),
                    worker_replies: HashSet::new(),
                };
                for worker in self.worker_refs.iter() {
                    worker.tell(WithSender::from(WorkerMessage::ApplyDrp(alpha), self));
                }
            }
            _ => panic!("an outstanding request is already in progress"),
        }
    }

    fn handle_worker_drp_complete(&mut self, worker_idx: usize) {
        println!(
            "manager: received WorkerDrpComplete message from worker {}",
            worker_idx
        );
        match &mut self.request {
            ManagerRequest::ApplyDrp {
                request,
                worker_replies,
            } => {
                worker_replies.insert(worker_idx);
                if worker_replies.len() == self.num_workers {
                    let request = request.take().expect("request");
                    request.reply(()).unwrap();
                }
            }
            _ => panic!("ApplyDrp request is not in progress"),
        }
    }

    // RETRIEVE REMAINDER WORKFLOW
    // --------------------------------------------------------------------------------------------
    fn handle_retrieve_remainder(&mut self, msg: Ask<(), Vec<BaseElement>>) {
        println!("manager: received RetrieveRemainder message");
        match self.request {
            ManagerRequest::None => {
                self.request = ManagerRequest::RetrieveRemainder {
                    request: Some(msg),
                    worker_remainders: BTreeMap::new(),
                };
                for worker in self.worker_refs.iter() {
                    worker.tell(WithSender::from(WorkerMessage::RetrieveRemainder, self));
                }
            }
            _ => panic!("an outstanding request is already in progress"),
        }
    }

    fn handle_worker_remainder(&mut self, worker_idx: usize, remainder: BaseElement) {
        println!(
            "manager: received WorkerRemainder message from worker {}",
            worker_idx
        );
        match &mut self.request {
            ManagerRequest::RetrieveRemainder {
                request,
                worker_remainders,
            } => {
                worker_remainders.insert(worker_idx, remainder);
                if worker_remainders.len() == self.num_workers {
                    let request = request.take().expect("request");
                    request
                        .reply(worker_remainders.values().cloned().collect())
                        .unwrap();
                }
            }
            _ => panic!("RetrieveRemainder request is not in progress"),
        }
    }

    // QUERY LAYERS WORKFLOW
    // --------------------------------------------------------------------------------------------
    fn handle_query_layers(&mut self, msg: Ask<Vec<usize>, Vec<(usize, Vec<Vec<QueryResult>>)>>) {
        println!("manager: received QueryLayers message");
        match self.request {
            ManagerRequest::None => {
                let positions = msg.request().clone();
                self.request = ManagerRequest::QueryLayers {
                    request: Some(msg),
                    worker_queries: BTreeMap::new(),
                };
                for worker in self.worker_refs.iter() {
                    let msg = WorkerMessage::Query(positions.clone());
                    worker.tell(WithSender::from(msg, self));
                }
            }
            _ => panic!("an outstanding request is already in progress"),
        }
    }

    fn handle_worker_query_result(&mut self, worker_idx: usize, queries: Vec<Vec<QueryResult>>) {
        println!(
            "manager: received WorkerQueryResult message from worker {}",
            worker_idx
        );
        match &mut self.request {
            ManagerRequest::QueryLayers {
                request,
                worker_queries,
            } => {
                worker_queries.insert(worker_idx, queries);
                if worker_queries.len() == self.num_workers {
                    let request = request.take().expect("request");
                    request
                        .reply(
                            worker_queries
                                .into_iter()
                                .map(|(&i, q)| (i, q.clone()))
                                .collect(),
                        )
                        .unwrap();
                }
            }
            _ => panic!("RetrieveRemainder request is not in progress"),
        }
    }
}

// ACTOR IMPLEMENTATION
// ================================================================================================

impl Actor for Manager {
    type Message = ManagerMessage;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg {
            // distribute evaluations workflow ----------------------------------------------------
            ManagerMessage::DistributeEvaluations(msg) => self.handle_distribute_evaluations(msg),
            ManagerMessage::WorkerReady(worker_idx) => self.handle_worker_ready(worker_idx),
            // layer commitment workflow ----------------------------------------------------------
            ManagerMessage::CommitToLayer(msg) => self.handle_commit_to_layer(msg),
            ManagerMessage::WorkerCommit(worker_idx, commitment) => {
                self.handle_worker_commit(worker_idx, commitment)
            }
            // degree-respecting projection workflow ----------------------------------------------
            ManagerMessage::ApplyDrp(msg) => self.handle_apply_drp(msg),
            ManagerMessage::WorkerDrpComplete(worker_idx) => {
                self.handle_worker_drp_complete(worker_idx)
            }
            // remainder retrieval workflow -------------------------------------------------------
            ManagerMessage::RetrieveRemainder(msg) => self.handle_retrieve_remainder(msg),
            ManagerMessage::WorkerRemainder(worker_idx, remainder) => {
                self.handle_worker_remainder(worker_idx, remainder)
            }
            // layer query workflow ---------------------------------------------------------------
            ManagerMessage::QueryLayers(msg) => self.handle_query_layers(msg),
            ManagerMessage::WorkerQueryResult(worker_idx, queries) => {
                self.handle_worker_query_result(worker_idx, queries)
            }
        }

        if self.request.is_handled() {
            self.request = ManagerRequest::None;
        }

        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}

impl ComponentLifecycle for Manager {
    fn on_start(&mut self) -> Handled {
        // set up our workers
        for i in 0..self.num_workers {
            let config = WorkerConfig {
                num_partitions: self.num_workers,
                index: i,
                hash_fn: crypto::hash::blake3,
            };
            let worker = self.ctx.system().create(|| Worker::new(config));
            let worker_ref = worker.actor_ref().hold().expect("live");
            self.ctx.system().start(&worker);
            self.workers.push(worker);
            self.worker_refs.push(worker_ref);
        }
        Handled::Ok
    }

    fn on_stop(&mut self) -> Handled {
        // clean up after ourselves
        self.worker_refs.clear();
        let system = self.ctx.system();
        self.workers.drain(..).for_each(|worker| {
            system.stop(&worker);
        });
        Handled::Ok
    }

    fn on_kill(&mut self) -> Handled {
        self.on_stop()
    }
}

// REQUESTS
// ================================================================================================

enum ManagerRequest {
    DistributeEvaluations {
        request: Option<Ask<Arc<Vec<BaseElement>>, ()>>,
        worker_replies: HashSet<usize>,
    },
    CommitToLayer {
        request: Option<Ask<(), Vec<[u8; 32]>>>,
        worker_commitments: BTreeMap<usize, [u8; 32]>,
    },
    ApplyDrp {
        request: Option<Ask<BaseElement, ()>>,
        worker_replies: HashSet<usize>,
    },
    RetrieveRemainder {
        request: Option<Ask<(), Vec<BaseElement>>>,
        worker_remainders: BTreeMap<usize, BaseElement>,
    },
    QueryLayers {
        request: Option<Ask<Vec<usize>, Vec<(usize, Vec<Vec<QueryResult>>)>>>,
        worker_queries: BTreeMap<usize, Vec<Vec<QueryResult>>>,
    },
    None,
}

impl ManagerRequest {
    pub fn is_handled(&self) -> bool {
        match self {
            ManagerRequest::DistributeEvaluations { request, .. } => request.is_none(),
            ManagerRequest::CommitToLayer { request, .. } => request.is_none(),
            ManagerRequest::ApplyDrp { request, .. } => request.is_none(),
            ManagerRequest::RetrieveRemainder { request, .. } => request.is_none(),
            ManagerRequest::QueryLayers { request, .. } => request.is_none(),
            ManagerRequest::None => true,
        }
    }
}
