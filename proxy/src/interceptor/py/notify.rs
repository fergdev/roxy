use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

use once_cell::sync::OnceCell;
use pyo3::{PyResult, exceptions::PyRuntimeError, pyfunction};

use crate::interceptor::FlowNotify;

static NOTIFY_TX: OnceCell<Arc<Mutex<Option<Sender<FlowNotify>>>>> = OnceCell::new();

pub(crate) async fn init_notify(tx: Option<Sender<FlowNotify>>) {
    let notify_tx = NOTIFY_TX.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut g = notify_tx.lock().await;
    *g = tx;
}

#[pyfunction]
#[pyo3(signature = (level, message))]
pub(crate) fn notify(level: i32, message: &str) -> PyResult<()> {
    let message = message.to_owned();
    tokio::spawn(async move {
        if let Some(tx) = NOTIFY_TX.get() {
            let g = tx.lock().await;
            // .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
            if let Some(ref tx) = *g {
                tx.send(FlowNotify::new(level.into(), message.to_string()))
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("notify send failed: {e}")))?;
            }
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("notify not initialized"))
        }
    });
    Ok(())
}
