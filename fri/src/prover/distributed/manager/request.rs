use kompact::prelude::*;
use math::field::BaseElement;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::prover::distributed::{messages::RequestInfo, ProtocolStep};

use super::{Manager, QueryResult, WorkerPartitions, WorkerRequest};

// REQUEST
// ================================================================================================

pub struct RequestState {
    num_partitions: usize,
    workers_partitions: HashMap<ActorPath, Vec<usize>>,
    state: ProtocolStep,
    current_procedure: Procedure,
}

impl RequestState {
    // INIT REQUEST WORKFLOW
    // --------------------------------------------------------------------------------------------

    pub fn init(
        request_info: RequestInfo,
        workers: Vec<ActorPath>,
        manager: &mut Manager,
        response: KPromise<()>,
    ) -> Self {
        let RequestInfo {
            num_partitions,
            num_layers,
            evaluations,
        } = request_info;

        let mut workers_partitions = HashMap::new();
        let partitions_per_worker = num_partitions / workers.len();

        // iterate over workers and assign a disjoin set of partitions to each worker
        for (i, worker) in workers.iter().enumerate() {
            let partition_indexes: Vec<usize> = (0..num_partitions)
                .skip(i * partitions_per_worker)
                .take(partitions_per_worker)
                .collect();
            workers_partitions.insert(worker.clone(), partition_indexes.clone());

            let wp = WorkerPartitions {
                evaluations: evaluations.to_vec(),
                num_partitions,
                partition_indexes,
                num_layers,
            };
            worker.tell(WorkerRequest::AssignPartitions(wp), manager);
        }

        // set the current procedure (we are still in the init phase)
        let current_procedure = Procedure::InitRequest {
            response: Some(response),
            done_workers: HashSet::new(),
        };

        // once the init procedure completes, the state will be Ready
        RequestState {
            num_partitions,
            workers_partitions,
            state: ProtocolStep::Ready(num_layers),
            current_procedure,
        }
    }

    pub fn handle_worker_ready(&mut self, sender: ActorPath) {
        if let Procedure::InitRequest {
            response,
            done_workers,
        } = &mut self.current_procedure
        {
            // keep track of which workers are ready
            done_workers.insert(sender);

            // when all workers are ready, fulfill the response
            if done_workers.len() == self.workers_partitions.len() {
                response.take().unwrap().fulfil(()).unwrap();
                self.current_procedure = Procedure::None;
            }
        } else {
            panic!("cannot process worker ready: InitRequest procedure is not in progress");
        }
    }

    // COMMIT TO LAYER WORKFLOW
    // --------------------------------------------------------------------------------------------

    pub fn commit_to_layer(&mut self, manger: &mut Manager, response: KPromise<Vec<[u8; 32]>>) {
        // make sure there is no procedure currently in progress
        assert!(
            self.current_procedure.is_none(),
            "cannot start CommitToLayer procedure; another procedure is currently in progress"
        );

        // make sure CommitToLayer procedure can be invoked at this point in the protocol
        let next_state = self.state.get_next_state();
        if let ProtocolStep::LayerCommitted(_) = next_state {
            self.state = next_state;
        } else {
            panic!(
                "transition from {:?} to {:?} is not valid",
                self.state, next_state
            );
        }

        // dispatch CommitToLayer to layer messages to all workers and update current procedure
        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::CommitToLayer, manger);
        }
        self.current_procedure = Procedure::CommitToLayer {
            response: Some(response),
            commitments: BTreeMap::new(),
        };
    }

    #[rustfmt::skip]
    pub fn handle_worker_commit(&mut self, worker: ActorPath, worker_commitments: Vec<[u8; 32]>) {
        if let Procedure::CommitToLayer {response, commitments} = &mut self.current_procedure {
            // insert commitments received from workers into commitment accumulator
            let partitions = self.workers_partitions.get(&worker).unwrap();
            for (&idx, &rem) in partitions.iter().zip(worker_commitments.iter()) {
                commitments.insert(idx, rem);
            }

            // when all commitments were received, fulfill the response
            if commitments.len() == self.num_partitions {
                let commitments = commitments.values().cloned().collect();
                response.take().unwrap().fulfil(commitments).unwrap();
                self.current_procedure = Procedure::None;
            }
        } else {
            panic!("cannot process worker commitment: CommitToLayer procedure is not in progress");
        }
    }

    // APPLY DRP WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn apply_drp(&mut self, alpha: BaseElement, manger: &mut Manager, response: KPromise<()>) {
        // make sure there is no procedure currently in progress
        assert!(
            self.current_procedure.is_none(),
            "cannot start ApplyDrp procedure; another procedure is currently in progress"
        );

        // make sure ApplyDrp procedure can be invoked at this point in the protocol
        let next_state = self.state.get_next_state();
        if let ProtocolStep::DrpApplied(_) = next_state {
            self.state = next_state;
        } else {
            panic!(
                "transition from {:?} to {:?} is not valid",
                self.state, next_state
            );
        }

        // dispatch ApplyDrp messages to all workers and update current procedure
        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::ApplyDrp(alpha), manger);
        }
        self.current_procedure = Procedure::ApplyDrp {
            response: Some(response),
            done_workers: HashSet::new(),
        };
    }

    #[rustfmt::skip]
    pub fn handle_worker_drp_complete(&mut self, sender: ActorPath) {
        if let Procedure::ApplyDrp {response, done_workers} = &mut self.current_procedure {
            // keep track of which workers have completed the procedure
            done_workers.insert(sender);

            // when all workers are done, fulfill the response
            if done_workers.len() == self.workers_partitions.len() {
                response.take().unwrap().fulfil(()).unwrap();
                self.current_procedure = Procedure::None;
            }
        } else {
            panic!("cannot process worker DRP complete: ApplyDrp procedure is not in progress");
        }
    }

    // RETRIEVE REMAINDER WORKFLOW
    // --------------------------------------------------------------------------------------------
    pub fn retrieve_remainder(
        &mut self,
        manger: &mut Manager,
        response: KPromise<Vec<BaseElement>>,
    ) {
        // make sure there is no procedure currently in progress
        assert!(
            self.current_procedure.is_none(),
            "cannot start RetrieveRemainder procedure; another procedure is currently in progress"
        );

        // make sure layer reduction is finished
        assert!(
            self.state.is_completed(),
            "cannot retrieve remainder: layer reduction hasn't been completed yet"
        );

        // dispatch RetrieveRemainder message to all works and update current procedure
        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::RetrieveRemainder, manger);
        }
        self.current_procedure = Procedure::RetrieveRemainder {
            response: Some(response),
            remainder: BTreeMap::new(),
        };
    }

    #[rustfmt::skip]
    pub fn handle_worker_remainder(
        &mut self,
        worker: ActorPath,
        worker_remainder: Vec<BaseElement>,
    ) {
        if let Procedure::RetrieveRemainder {response, remainder} = &mut self.current_procedure {
            // insert remainders from the worker into remainder accumulator
            let partitions = self.workers_partitions.get(&worker).unwrap();
            for (&idx, &rem) in partitions.iter().zip(worker_remainder.iter()) {
                remainder.insert(idx, rem);
            }

            // when all remainders have been received, fulfill the response
            if remainder.len() == self.num_partitions {
                let remainder = remainder.values().cloned().collect();
                response.take().unwrap().fulfil(remainder).unwrap();
                self.current_procedure = Procedure::None;
            }
        } else {
            panic!("cannot process remainder result: RetrieveRemainder procedure is not in progress");
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
        // make sure there is no procedure currently in progress
        assert!(
            self.current_procedure.is_none(),
            "cannot start QueryLayers procedure; another procedure is currently in progress"
        );

        // make sure layer reduction is finished
        assert!(
            self.state.is_completed(),
            "cannot query layers: layer reduction hasn't been completed yet"
        );

        // dispatch Query message to all workers and update current procedure
        for (worker, _) in self.workers_partitions.iter() {
            worker.tell(WorkerRequest::Query(positions.clone()), manger);
        }
        self.current_procedure = Procedure::QueryLayers {
            response: Some(response),
            query_results: BTreeMap::new(),
        };
    }

    #[rustfmt::skip]
    pub fn handle_worker_query_result(
        &mut self,
        worker: ActorPath,
        worker_results: Vec<Vec<Vec<QueryResult>>>,
    ) {
        if let Procedure::QueryLayers {response, query_results} = &mut self.current_procedure {
            // insert query results from the worker into query result accumulator
            let partitions = self.workers_partitions.get(&worker).unwrap();
            for (&idx, res) in partitions.iter().zip(worker_results.into_iter()) {
                query_results.insert(idx, res);
            }

            // when all results have been received, fulfill the response
            if query_results.len() == self.num_partitions {
                let remainder = query_results.values().cloned().collect();
                response.take().unwrap().fulfil(remainder).unwrap();
                self.current_procedure = Procedure::None;
            }
        } else {
            panic!("cannot process query result: QueryLayers procedure is not in progress");
        }
    }
}

// PROCEDURE
// ================================================================================================

enum Procedure {
    InitRequest {
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

impl Procedure {
    pub fn is_none(&self) -> bool {
        match self {
            Procedure::None => true,
            _ => false,
        }
    }
}
