use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyDict, PyList},
};
use std::{ffi::CString, sync::Arc};

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::{KEY_REQUEST, KEY_RESPONSE, py::init_python},
};

use async_trait::async_trait;
use pyo3::ffi::c_str;
use tokio::sync::{Mutex, mpsc::Sender};
use tracing::{error, trace};

use crate::interceptor::{Error, FlowNotify, KEY_EXTENSIONS, RoxyEngine, py::flow::PyFlow};

#[derive(Debug, Clone)]
pub(crate) struct PythonEngine {
    addons: Arc<Mutex<Vec<PyAddon>>>,
}

impl PythonEngine {
    pub fn new(notify_tx: Option<Sender<FlowNotify>>) -> Self {
        init_python();
        if let Some(notify_tx) = notify_tx {
            Python::attach(|py| {
                let _ = inject_notify(py, notify_tx);
            });
        }
        Self {
            addons: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
#[pyclass]
struct Notifier {
    tx: Sender<FlowNotify>,
}

#[pymethods]
impl Notifier {
    #[pyo3(name = "__call__")]
    fn __call__(&self, level: i32, msg: String) -> PyResult<()> {
        let _ = self.tx.try_send(FlowNotify {
            level: level.into(),
            msg,
        });
        Ok(())
    }

    fn notify(&self, level: i32, msg: String) -> PyResult<()> {
        self.__call__(level, msg)
    }
}

fn inject_notify(py: Python<'_>, tx: Sender<FlowNotify>) -> PyResult<()> {
    let notifier = Py::new(py, Notifier { tx })?;
    let builtins = py.import("builtins")?;
    builtins.setattr("notify", notifier.clone_ref(py))?;
    let roxy = PyModule::new(py, "Roxy")?;
    roxy.add("notify", notifier)?;
    let sys = py.import("sys")?;
    sys.getattr("modules")?
        .downcast::<PyDict>()?
        .set_item("Roxy", roxy)?;
    Ok(())
}

impl Default for PythonEngine {
    fn default() -> Self {
        Python::initialize();
        Self::new(None)
    }
}

#[async_trait]
impl RoxyEngine for PythonEngine {
    async fn intercept_request(
        &self,
        req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error> {
        let addons = self.addons.lock().await;
        Python::attach(|py| {
            let f = PyFlow::from_data(py, req, &None)?;
            let flow_obj = f.bind(py);
            for a in addons.iter() {
                let obj = a.obj.bind(py);
                if let Err(err) = obj.call_method(KEY_REQUEST, (&flow_obj,), None) {
                    error!("Addon `{}` error in `intercept_request`: {}", a.name, err);
                }
            }
            update_request(flow_obj, req)
        })
    }

    async fn intercept_response(&self, res: &mut InterceptedResponse) -> Result<(), Error> {
        let addons = self.addons.lock().await;
        Python::attach(|py| {
            let f = PyFlow::from_data(py, &InterceptedRequest::default(), &Some(res.clone()))?;
            let flow_obj = f.bind(py);
            for a in addons.iter() {
                let obj = a.obj.bind(py);
                if let Err(err) = obj.call_method(KEY_RESPONSE, (&flow_obj,), None) {
                    error!("Addon `{}` error in `intercept_response`: {}", a.name, err);
                }
            }
            update_response(flow_obj, res)?;
            Ok(())
        })
    }

    async fn set_script(&self, script: &str) -> Result<(), Error> {
        let mut guard = self.addons.lock().await;
        trace!("Setting python script {}\n{script:?}", guard.len());
        guard.clear();
        drop(guard);

        let new_addons = Python::attach(|py| {
            let module = PyModule::from_code(
                py,
                CString::new(script)
                    .map_err(|_| Error::Other("Could not convert to c_str".into()))?
                    .as_c_str(),
                c_str!("roxyscript.py"),
                c_str!("roxyscript"),
            )?;

            let addons_obj = match module.getattr(KEY_EXTENSIONS) {
                Ok(ext) => ext,
                Err(e) => {
                    error!("addons {e}");
                    return Ok(vec![]);
                }
            };

            let addons_list: &Bound<'_, PyList> = if let Ok(lst) = addons_obj.cast::<PyList>() {
                lst
            } else {
                return Err(Error::Other("`addons` must be a list/tuple".into()));
            };

            let mut new_addons = Vec::with_capacity(addons_list.len());

            for item in addons_list.iter() {
                let name = item
                    .getattr("__class__")
                    .and_then(|cls| cls.getattr("__name__"))
                    .ok()
                    .and_then(|n| n.extract::<String>().ok())
                    .unwrap_or_else(|| "<addon>".into());

                let obj: Py<PyAny> = item.unbind();
                new_addons.push(PyAddon { name, obj });
            }
            Ok(new_addons)
        })?;
        self.addons.lock().await.extend(new_addons);
        Ok(())
    }
}

fn update_request<'py>(
    flow_obj: &Bound<'py, PyAny>,
    req: &mut InterceptedRequest,
) -> Result<Option<InterceptedResponse>, Error> {
    let flow_cell = flow_obj
        .downcast::<PyFlow>()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?;

    let py_req = &flow_cell.borrow().request;
    req.version = py_req
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .version;
    req.uri = py_req
        .url
        .uri
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .clone();

    req.method = py_req
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .method
        .clone();

    req.body = py_req
        .body
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .clone();

    req.headers = py_req
        .headers
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .clone();
    req.trailers = {
        let t = py_req
            .trailers
            .inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("{e}")))?
            .clone();
        if t.is_empty() { None } else { Some(t) }
    };

    let mut resp = InterceptedResponse::default();
    update_response(flow_obj, &mut resp)?;
    if (resp.status != 0)
        || (!resp.body.is_empty())
        || (!resp.headers.is_empty())
        || (resp.trailers.is_some())
    {
        return Ok(Some(resp));
    }

    Ok(None)
}

fn update_response<'py>(
    flow_obj: &Bound<'py, PyAny>,
    res: &mut InterceptedResponse,
) -> Result<(), Error> {
    let flow_cell = flow_obj
        .downcast::<PyFlow>()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?;

    let resp = flow_cell.borrow();
    let resp = &resp.response;
    let resp = resp
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?;
    res.body = flow_cell
        .borrow()
        .response
        .body
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .clone();
    res.status = resp.status;
    res.version = resp.version;
    res.headers = flow_cell
        .borrow()
        .response
        .headers
        .inner
        .lock()
        .map_err(|e| PyTypeError::new_err(format!("{e}")))?
        .clone();
    res.trailers = {
        let t = flow_cell
            .borrow()
            .response
            .trailers
            .inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("{e}")))?
            .clone();
        if t.is_empty() { None } else { Some(t) }
    };

    Ok(())
}

#[derive(Debug)]
struct PyAddon {
    name: String,
    obj: Py<PyAny>,
}
