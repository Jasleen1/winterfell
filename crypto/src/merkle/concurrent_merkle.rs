use crate::HashFunction;
use bytes::BytesMut;
use kompact::prelude::*;
use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::slice;

use std::{fmt, ops::Range, sync::Arc};

pub struct Work {
    data: Arc<[[u8; 32]]>,
    hasher: HashFunction,
}

impl Work {
    pub fn with(leaves: &[[u8; 32]], hasher: HashFunction) -> Self {
        let leaves_data: Arc<[[u8; 32]]> = leaves.into();
        Work {
            data: leaves_data,
            hasher,
        }
    }
}
impl fmt::Debug for Work {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Work{{
            data=<data of length={}>,
            hasher=<function>,
        }}",
            self.data.len(),
        )
    }
}

struct WorkPart {
    data: Arc<[[u8; 32]]>,
    range: Range<usize>,
    hasher: HashFunction,
    output_buffers: Vec<BytesMut>,
}

impl WorkPart {
    fn from(work: &Work, range: Range<usize>, output_buffers: Vec<BytesMut>) -> Self {
        WorkPart {
            data: work.data.clone(),
            hasher: work.hasher,
            range,
            output_buffers,
        }
    }
}
impl fmt::Debug for WorkPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WorkPart{{
            data=<data of length={}>,
            range={:?},
            buffers=<buffers of length={}>,
        }}",
            self.data.len(),
            self.range,
            self.output_buffers.len(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkResult(Vec<BytesMut>, Range<usize>);

// The result_accumulator queue depends on `Ord`.
// its' a max-heap: the last few elements will be first
impl Ord for WorkResult {
    fn cmp(&self, other: &WorkResult) -> Ordering {
        self.1.start.cmp(&other.1.start)
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for WorkResult {
    fn partial_cmp(&self, other: &WorkResult) -> Option<Ordering> {
        Some(<WorkResult as Ord>::cmp(self, other))
    }
}

struct WorkerPort;
impl Port for WorkerPort {
    type Indication = WorkResult;
    type Request = Never;
}

#[derive(Clone, Debug)]
pub struct FinalWorkResult(pub Vec<[u8; 32]>);

#[derive(ComponentDefinition)]
pub struct Manager {
    ctx: ComponentContext<Self>,
    worker_port: RequiredPort<WorkerPort>,
    num_workers: usize,
    workers: Vec<Arc<Component<Worker>>>,
    worker_refs: Vec<ActorRefStrong<WorkPart>>,
    outstanding_request: Option<Ask<Work, FinalWorkResult>>,
    top_elements: Option<BytesMut>,
    result_accumulator: BinaryHeap<WorkResult>,
}

impl Manager {
    pub fn new(num_workers: usize) -> Self {
        Manager {
            ctx: ComponentContext::uninitialised(),
            worker_port: RequiredPort::uninitialised(),
            num_workers,
            workers: Vec::with_capacity(num_workers),
            worker_refs: Vec::with_capacity(num_workers),
            outstanding_request: None,
            top_elements: None,
            result_accumulator: BinaryHeap::with_capacity(num_workers + 1),
        }
    }
}

impl ComponentLifecycle for Manager {
    fn on_start(&mut self) -> Handled {
        assert!(is_power_of_two(self.num_workers + 1), "The number of worker threads + 1 should be a power of two to ensure even division of work");
        // set up our workers
        for _i in 0..self.num_workers {
            let worker = self.ctx.system().create(Worker::new);
            worker.connect_to_required(self.worker_port.share());
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

impl Require<WorkerPort> for Manager {
    fn handle(&mut self, event: WorkResult) -> Handled {
        if self.outstanding_request.is_some() {
            self.result_accumulator.push(event);
            if self.result_accumulator.len() == (self.num_workers + 1) {
                let ask = self.outstanding_request.take().expect("ask");
                let work = ask.request();

                let res: Vec<Vec<BytesMut>> =
                    std::mem::replace(&mut self.result_accumulator, BinaryHeap::new())
                        .into_iter()
                        .map(|WorkResult(buffers, _range)| buffers)
                        .collect();
                debug_assert!(res.len() == self.num_workers + 1);

                // assemble the contributions into a level by level Vec
                let bytes_per_level: Vec<BytesMut> = res
                    .into_iter()
                    .fold_first(|acc, elem| {
                        elem.into_iter()
                            .zip(acc)
                            .map(|(mut e, a)| {
                                e.unsplit(a);
                                e
                            })
                            .collect()
                    })
                    .expect(
                        "BytesMut from (self.num_workers + 1) WorkResults should always be nonempty.",
                    );
                debug!(
                    self.log(),
                    "Aggregated {:?} contributions leading to {} levels",
                    self.num_workers + 1,
                    bytes_per_level.len()
                );

                let large_array: BytesMut = bytes_per_level
                    .into_iter()
                    .fold_first(|acc, mut elem| {
                        elem.unsplit(acc);
                        elem
                    })
                    .expect("distributed Levels on initial work should be nonempty");

                // Now we have everything but the last self.num_workers+1 last elements
                let mut init_array = std::mem::replace(&mut self.top_elements, None)
                    .expect("init_array should be set up during manager's receive_local");
                init_array.unsplit(large_array);
                let res: &mut [u8] = init_array.as_mut();
                let n = &work.data.len() / 2;
                let two_nodes =
                    unsafe { slice::from_raw_parts(res.as_ptr() as *const [u8; 64], n) };
                let nodes =
                    unsafe { slice::from_raw_parts_mut(res.as_ptr() as *mut [u8; 32], 2 * n) };

                for i in (1..self.num_workers + 1).rev() {
                    (work.hasher)(&two_nodes[i], &mut nodes[i]);
                }

                let reply = FinalWorkResult(nodes.to_vec());
                ask.reply(reply).expect("reply");
            }
        } else {
            error!(
                self.log(),
                "Got a response without an outstanding promise: {:?}", event
            );
        }
        Handled::Ok
    }
}

impl Actor for Manager {
    type Message = Ask<Work, FinalWorkResult>;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        assert!(self.outstanding_request.is_none(), "One request at a time!");
        let work: &Work = msg.request();
        if self.num_workers == 0 {
            // manager gotta work itself -> very unhappy manager
            let res = super::build_merkle_nodes(&work.data, work.hasher);
            msg.reply(FinalWorkResult(res.into())).expect("reply");
        } else {
            let len = work.data.len();
            // The task should be evenly split between workers and the manager
            assert!(len % (self.num_workers + 1) == 0);
            let stride = len / (self.num_workers + 1);
            let mut buffer = BytesMut::with_capacity(len * 32);
            unsafe {
                buffer.set_len(len * 32);
            }

            debug!(
                self.log(),
                "Preparing output splits of output buffer of length {:?} into #{}",
                buffer.len(),
                self.num_workers + 1
            );

            let all_output_buffers = split_off_buffers(&mut buffer, 32, self.num_workers + 1);
            self.top_elements = Some(buffer);

            all_output_buffers
                .into_iter()
                .enumerate()
                .for_each(|(index, out_buffers)| {
                    let start = 0usize + stride * index;
                    if start < len && index < self.num_workers {
                        let end = len.min(start + stride);
                        let range = start..end;
                        debug!(self.log(), "Assigning {:?} to worker #{}", range, index,);
                        let msg = WorkPart::from(work, range, out_buffers);
                        let worker = &self.worker_refs[index];
                        worker.tell(msg);
                    } else {
                        // manager just does the rest itself
                        let range = Range { start, end: len };
                        let written_bufs =
                            hash_all_levels(&work.data, &range, work.hasher, out_buffers);
                        self.result_accumulator
                            .push(WorkResult(written_bufs, range));
                    }
                });
            self.outstanding_request = Some(msg);
        }
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Still ignoring networking stuff.");
    }
}

fn is_power_of_two(n: usize) -> bool {
    n & (n - 1) == 0
}

// TODO: give a real docstring to this function that splits a binary tree in num_splits
fn split_off_buffers(
    main_buf: &mut BytesMut,
    element_stride: usize,
    num_splits: usize,
) -> Vec<Vec<BytesMut>> {
    let n = main_buf.len() / element_stride;
    // check n is a power of two
    assert!(
        is_power_of_two(n),
        "buffer should contain a power of two of stride-sized elements!"
    );
    let mut res: Vec<Vec<BytesMut>> = vec![Vec::new(); num_splits];
    // the first num_splits elements, the "tip" is left in the original
    // reference for the manager finish assembly
    while main_buf.len() / element_stride > num_splits {
        let midpoint = main_buf.len() / 2;
        let mut this_level = main_buf.split_off(midpoint);
        // split this_level in num_splits equal parts that we push in res
        let split_stride = this_level.len() / num_splits;
        for i in (1..num_splits).rev() {
            let chunk = this_level.split_off(this_level.len() - split_stride);
            res[i].push(chunk);
        }
        res[0].push(this_level);
    }
    debug_assert_eq!(
        res[0].len(),
        (n.trailing_zeros() - num_splits.trailing_zeros()) as usize
    );
    res
}

#[derive(ComponentDefinition)]
struct Worker {
    ctx: ComponentContext<Self>,
    worker_port: ProvidedPort<WorkerPort>,
}
impl Worker {
    fn new() -> Self {
        Worker {
            ctx: ComponentContext::uninitialised(),
            worker_port: ProvidedPort::uninitialised(),
        }
    }
}

ignore_lifecycle!(Worker);
ignore_requests!(WorkerPort, Worker);

impl Actor for Worker {
    type Message = WorkPart;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        let out_len = msg.output_buffers.len();
        let range = msg.range.clone();
        let written_buffers =
            hash_all_levels(&msg.data, &msg.range, msg.hasher, msg.output_buffers);
        debug!(
            self.log(),
            "Returning {} slices (of {}) for range {:?}",
            written_buffers.len(),
            out_len,
            range,
        );
        self.worker_port
            .trigger(WorkResult(written_buffers, msg.range));
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("ignoring network");
    }
}

fn hash_all_levels(
    data: &[[u8; 32]],
    range: &Range<usize>,
    hasher: HashFunction,
    output_buffers: Vec<BytesMut>,
) -> Vec<BytesMut> {
    debug_assert!(!range.is_empty());
    let mut read_slice: &[[u8; 32]] = data;

    let res: Vec<BytesMut> = output_buffers
        .into_iter()
        .enumerate()
        .map(|(idx, mut write_slice)| {
            let total_range = Range {
                start: 0,
                end: read_slice.len(),
            };

            let rng = if idx == 0 { range } else { &total_range };

            hash_a_level(&read_slice, rng, hasher, &mut write_slice);

            read_slice = unsafe {
                slice::from_raw_parts(
                    write_slice.as_ref().as_ptr() as *const [u8; 32],
                    (rng.end - rng.start) / 2,
                )
            };
            write_slice
        })
        .collect();
    res
}

// in this occasion our behavior is purely functional, but note the receive local
// function has a mutable reference to its actor, we could hold state within it
fn hash_a_level(
    data: &[[u8; 32]],
    range: &Range<usize>,
    hasher: HashFunction,
    output: &mut BytesMut,
) {
    let n = (range.end - range.start) / 2;
    let out_len = output.len() / 32;
    debug_assert_eq!(out_len, n);

    let slice = &data[range.clone()];

    let two_slice = unsafe { slice::from_raw_parts(slice.as_ptr() as *const [u8; 64], n) };

    let out_slice =
        unsafe { slice::from_raw_parts_mut(output.as_mut().as_ptr() as *mut [u8; 32], n) };

    for i in 0..n {
        hasher(&two_slice[i], &mut out_slice[i]);
    }
}

pub fn build_merkle_nodes(
    num_workers: usize,
    leaves: &[[u8; 32]],
    hasher: HashFunction,
) -> Vec<[u8; 32]> {
    let system = KompactConfig::default().build().expect("system");
    let manager = system.create(move || Manager::new(num_workers));
    system.start(&manager);
    let manager_ref = manager.actor_ref().hold().expect("live");

    let work = Work::with(leaves, hasher);
    let nodes: Vec<[u8; 32]> = manager_ref.ask(Ask::of(work)).wait().0;
    system.shutdown().expect("shutdown");
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use rand::{self, RngCore};

    #[test]
    fn test_split_off() {
        let mut buf = BytesMut::with_capacity(64 * 32);
        unsafe {
            buf.set_len(64 * 32);
        }

        let res = split_off_buffers(&mut buf, 32, 2);

        // two splits
        assert_eq!(res.len(), 2);
        // the last (num_splits - 1) level is not in the output buffers
        assert_eq!(res[0].len(), 5);
    }

    #[test]
    fn test_no_workers() {
        let mut rng = rand::thread_rng();

        let data: Vec<[u8; 32]> = {
            let mut res = utils::uninit_vector(64);
            for i in 0..64 {
                let mut v = [0u8; 32];
                rng.fill_bytes(&mut v);
                res[i] = v;
            }
            res
        };
        let res = build_merkle_nodes(0, &data, crate::hash::blake3);
        let seq_res = super::super::build_merkle_nodes(&data, crate::hash::blake3);
        assert_eq!(res, seq_res);
    }

    #[test]
    fn test_workers() {
        let mut rng = rand::thread_rng();

        let data: Vec<[u8; 32]> = {
            let mut res = utils::uninit_vector(128);
            for i in 0..128 {
                let mut v = [0u8; 32];
                rng.fill_bytes(&mut v);
                res[i] = v;
            }
            res
        };
        let res = build_merkle_nodes(3, &data, crate::hash::blake3);
        let seq_res = super::super::build_merkle_nodes(&data, crate::hash::blake3);
        assert_eq!(res, seq_res);
    }
}
