use pyo3::prelude::*;
use tracing::error;
use tracing::info;

#[pyclass]
pub(super) struct WriterStdOut;

#[pymethods]
impl WriterStdOut {
    fn write(&self, s: String) {
        if s.trim().is_empty() {
            return;
        }
        info!("[py] {s}");
    }
    fn flush(&self) {}
}

#[pyclass]
pub(super) struct WriterStdErr;

#[pymethods]
impl WriterStdErr {
    fn write(&self, s: String) {
        if s.trim().is_empty() {
            return;
        }
        error!("[py] {s}");
    }
    fn flush(&self) {}
}
