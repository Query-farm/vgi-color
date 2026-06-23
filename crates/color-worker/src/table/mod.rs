//! Table functions exposed by the color worker, registered under `color.main`.

mod named;

use vgi::Worker;

/// Register every table function on the worker.
pub fn register(worker: &mut Worker) {
    worker.register_table(named::NamedColors);
}
