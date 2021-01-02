use kompact::prelude::*;
use math::field::BaseElement;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use super::{Manager, QueryResult, WorkerPartitions, WorkerRequest};

pub struct RequestState {
    num_partitions: usize,
    workers_partitions: HashMap<ActorPath, Vec<usize>>,
    current_procedure: Procedure,
}

impl RequestState {
    pub fn new(workers: Vec<ActorPath>, num_partitions: usize) -> Self {
        let mut workers_partitions = HashMap::new();
        let partitions_per_worker = num_partitions / workers.len();

        for (i, worker) in workers.iter().enumerate() {
            let partition_indexes = (0..num_partitions)
                .skip(i * partitions_per_worker)
                .take(partitions_per_worker)
                .collect();
            workers_partitions.insert(worker.clone(), partition_indexes);
        }

        RequestState {
            num_partitions,
            workers_partitions,
            current_procedure: Procedure::None,
        }
    }

    // DISTRIBUTE EVALUATIONS WORKFLOW
    // --------------------------------------------------------------------------------------------

    pub fn distribute_evaluation(
        &mut self,
        evaluations: Arc<Vec<BaseElement>>,
        manger: &mut Manager,
        response: KPromise<()>,
    ) {
        // TODO: check state

        for (worker, partitions) in self.workers_partitions.iter() {
            let worker_partitions = WorkerPartitions {
                evaluations: evaluations.to_vec(),
                num_partitions: self.num_partitions,
                partition_indexes: partitions.clone(),
            };
            worker.tell(WorkerRequest::Prepare(worker_partitions), manger);
        }

        self.current_procedure = Procedure::DistributeEvaluations {
            response: Some(response),
            done_workers: HashSet::new(),
        };
    }

    pub fn handle_worker_ready(&mut self, sender: ActorPath) {
        // TODO: check state
        match &mut self.current_procedure {
            Procedure::DistributeEvaluations {
                response,
                done_workers,
            } => {
                done_workers.insert(sender);
                if done_workers.len() == self.workers_partitions.len() {
                    response.take().unwrap().fulfil(()).unwrap();
                }
            }
            _ => panic!("wrong procedure"),
        }
    }

    // COMMIT TO LAYER WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn commit_to_layer(&mut self, manger: &mut Manager, response: KPromise<Vec<[u8; 32]>>) {
        // TODO: check state

        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::CommitToLayer, manger);
        }

        self.current_procedure = Procedure::CommitToLayer {
            response: Some(response),
            commitments: BTreeMap::new(),
        };
    }

    pub fn handle_worker_commit(&mut self, worker: ActorPath, worker_commitments: Vec<[u8; 32]>) {
        // TODO: check state
        match &mut self.current_procedure {
            Procedure::CommitToLayer {
                response,
                commitments,
            } => {
                let partitions = self.workers_partitions.get(&worker).unwrap();
                for (&idx, &rem) in partitions.iter().zip(worker_commitments.iter()) {
                    commitments.insert(idx, rem);
                }

                if commitments.len() == self.num_partitions {
                    let commitments = commitments.values().cloned().collect();
                    response.take().unwrap().fulfil(commitments).unwrap();
                }
            }
            _ => panic!("wrong procedure"),
        }
    }

    // APPLY DRP WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn apply_drp(&mut self, alpha: BaseElement, manger: &mut Manager, response: KPromise<()>) {
        // TODO: check state

        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::ApplyDrp(alpha), manger);
        }

        self.current_procedure = Procedure::ApplyDrp {
            response: Some(response),
            done_workers: HashSet::new(),
        };
    }

    pub fn handle_worker_drp_complete(&mut self, sender: ActorPath) {
        // TODO: check state
        match &mut self.current_procedure {
            Procedure::ApplyDrp {
                response,
                done_workers,
            } => {
                done_workers.insert(sender);
                if done_workers.len() == self.workers_partitions.len() {
                    response.take().unwrap().fulfil(()).unwrap();
                }
            }
            _ => panic!("wrong procedure"),
        }
    }

    // RETRIEVE REMAINDER WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn retrieve_remainder(
        &mut self,
        manger: &mut Manager,
        response: KPromise<Vec<BaseElement>>,
    ) {
        // TODO: check state

        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::RetrieveRemainder, manger);
        }

        self.current_procedure = Procedure::RetrieveRemainder {
            response: Some(response),
            remainder: BTreeMap::new(),
        };
    }

    pub fn handle_worker_remainder(
        &mut self,
        worker: ActorPath,
        worker_remainder: Vec<BaseElement>,
    ) {
        // TODO: check state
        match &mut self.current_procedure {
            Procedure::RetrieveRemainder {
                response,
                remainder,
            } => {
                let partitions = self.workers_partitions.get(&worker).unwrap();
                for (&idx, &rem) in partitions.iter().zip(worker_remainder.iter()) {
                    remainder.insert(idx, rem);
                }

                if remainder.len() == self.num_partitions {
                    let remainder = remainder.values().cloned().collect();
                    response.take().unwrap().fulfil(remainder).unwrap();
                }
            }
            _ => panic!("wrong procedure"),
        }
    }

    // QUERY LAYERS WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn query_layers(
        &mut self,
        positions: Vec<usize>,
        manger: &mut Manager,
        response: KPromise<Vec<Vec<Vec<QueryResult>>>>,
    ) {
        // TODO: check state

        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::Query(positions.clone()), manger);
        }

        self.current_procedure = Procedure::QueryLayers {
            response: Some(response),
            query_results: BTreeMap::new(),
        };
    }

    pub fn handle_worker_query_result(
        &mut self,
        worker: ActorPath,
        worker_results: Vec<Vec<Vec<QueryResult>>>,
    ) {
        // TODO: check state
        match &mut self.current_procedure {
            Procedure::QueryLayers {
                response,
                query_results,
            } => {
                let partitions = self.workers_partitions.get(&worker).unwrap();
                for (&idx, res) in partitions.iter().zip(worker_results.into_iter()) {
                    query_results.insert(idx, res);
                }

                if query_results.len() == self.num_partitions {
                    let remainder = query_results.values().cloned().collect();
                    response.take().unwrap().fulfil(remainder).unwrap();
                }
            }
            _ => panic!("wrong procedure"),
        }
    }
}

enum Procedure {
    DistributeEvaluations {
        response: Option<KPromise<()>>,
        done_workers: HashSet<ActorPath>,
    },
    CommitToLayer {
        response: Option<KPromise<Vec<[u8; 32]>>>,
        commitments: BTreeMap<usize, [u8; 32]>,
    },
    ApplyDrp {
        response: Option<KPromise<()>>,
        done_workers: HashSet<ActorPath>,
    },
    RetrieveRemainder {
        response: Option<KPromise<Vec<BaseElement>>>,
        remainder: BTreeMap<usize, BaseElement>,
    },
    QueryLayers {
        response: Option<KPromise<Vec<Vec<Vec<QueryResult>>>>>,
        query_results: BTreeMap<usize, Vec<Vec<QueryResult>>>,
    },
    None,
}

// HELPER FUNCTIONS
// ================================================================================================
