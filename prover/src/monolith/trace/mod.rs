use super::{types::PolyTable, StarkDomain};

mod trace_table;
pub use trace_table::TraceTable;

mod execution_trace;
pub use execution_trace::ExecutionTrace;

#[cfg(test)]
mod tests;
