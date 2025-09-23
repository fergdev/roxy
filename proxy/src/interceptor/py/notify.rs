use std::ops::Deref;
use std::sync::Mutex;
use tokio::sync::mpsc::Sender;

use once_cell::sync::OnceCell;
use pyo3::{PyResult, exceptions::PyRuntimeError, pyfunction};
use tracing::{error, info};

use crate::interceptor::FlowNotify;

// TODO: set this module correctly when python is inited. correctly
// multi threads are breaking tests randomly, RUST_TEST_THREADS=1 solves the issue
static NOTIFY_TX: OnceCell<Mutex<Option<Sender<FlowNotify>>>> = OnceCell::new();

#[allow(clippy::expect_used)]
pub(crate) fn init_notify(tx: Option<Sender<FlowNotify>>) {
    info!("Init notify {}", tx.is_some());
    let notify_tx = NOTIFY_TX.get_or_init(|| Mutex::new(None));
    let mut g = notify_tx.lock().expect("Lock poisoned");
    *g = tx;
    info!("Init notify2 {}", g.is_some());
}

#[allow(clippy::expect_used)]
#[pyfunction]
#[pyo3(signature = (level, message))]
pub(crate) fn notify(level: i32, message: &str) -> PyResult<()> {
    info!("notify {level} {message}");
    let message = message.to_owned();
    if let Some(tx) = NOTIFY_TX.get() {
        info!("get");
        let g = tx.lock().expect("Lock poisoned");
        if let Some(tx) = g.deref() {
            info!("send");
            match tx
                .try_send(FlowNotify::new(level.into(), message.to_string()))
                .map_err(|e| PyRuntimeError::new_err(format!("notify send failed: {e}")))
            {
                Ok(_) => info!("send success"),
                Err(_) => error!("send failed"),
            }
        } else {
            info!("is none");
        }
        Ok(())
    } else {
        Err(PyRuntimeError::new_err("notify not initialized"))
    }
}
