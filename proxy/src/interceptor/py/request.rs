use std::{
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard},
};

use http::Method;
use pyo3::{
    Bound, PyAny, PyResult, exceptions::PyTypeError, pyclass, pymethods, types::PyAnyMethods,
};
use roxy_shared::version::HttpVersion;
use tracing::error;

use crate::{
    flow::InterceptedRequest,
    interceptor::py::{body::PyBody, constants::PyMethod, headers::PyHeaders, url::PyUrl},
};

#[derive(Debug, Clone)]
#[pyclass]
pub(crate) struct PyRequest {
    pub(crate) inner: Arc<Mutex<InterceptedRequest>>,
    pub(crate) method: Arc<Mutex<PyMethod>>,
    #[pyo3(get)]
    pub(crate) body: PyBody,
    #[pyo3(get)]
    pub(crate) url: PyUrl,
    #[pyo3(get)]
    pub(crate) headers: PyHeaders,
    #[pyo3(get)]
    pub(crate) trailers: PyHeaders,
}

impl Default for PyRequest {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InterceptedRequest::default())),
            method: Arc::new(Mutex::new(PyMethod::default())),
            body: PyBody::default(),
            url: PyUrl::default(),
            headers: PyHeaders::default(),
            trailers: PyHeaders::default(),
        }
    }
}

impl PyRequest {
    pub(crate) fn from_req(req: &InterceptedRequest) -> Self {
        PyRequest {
            inner: Arc::new(Mutex::new(req.clone())),
            method: Arc::new(Mutex::new(PyMethod::from(&req.method))),
            body: PyBody::new(req.body.clone()),
            url: PyUrl::from_ruri(req.uri.clone()),
            headers: PyHeaders::from_headers(req.headers.clone()),
            trailers: PyHeaders::from_headers(req.trailers.clone().unwrap_or_default()),
        }
    }
    fn lock(&self) -> PyResult<MutexGuard<'_, InterceptedRequest>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyRequest {
    #[new]
    fn new_py() -> Self {
        Self::default()
    }

    #[getter]
    fn method(&self) -> PyResult<PyMethod> {
        let g = self
            .method
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        error!("get method {}", g);
        Ok(g.clone())
    }
    #[setter]
    fn set_method(&mut self, py_val: Bound<PyAny>) -> PyResult<()> {
        error!("set method {:?}", py_val);
        let mut g = self
            .method
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        if let Ok(method) = py_val.extract::<PyMethod>() {
            error!("Assigning enum {method}");
            *g = method.clone();
            return Ok(());
        }

        if let Ok(s) = py_val.extract::<String>() {
            error!("Assigning string");
            *g = PyMethod::from(
                &Method::try_from(s.deref()).map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            return Ok(());
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "method must be Method enum or string",
        ))
    }

    #[getter]
    fn version(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.version.to_string())
    }
    #[setter]
    fn set_version(&self, value: &str) -> PyResult<()> {
        let value: HttpVersion = value
            .parse()
            .map_err(|_| PyTypeError::new_err(format!("Invalid HTTP version: {value}.")))?;
        let mut g = self.lock()?;
        g.version = value;
        Ok(())
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{self:?}"))
    }
    fn __repr__(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(format!("Request({:?})", g))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::py::with_module;

    #[test]
    fn pr01_constructor_exists() {
        with_module(
            r#"
from roxy import PyRequest
# Construct with defaults; just ensure it doesn't throw and exposes attributes
r = PyRequest()
assert hasattr(r, "method")
assert hasattr(r, "version")
"#,
        );
    }

    #[test]
    fn pr02_set_method_valid() {
        with_module(
            r#"
from roxy import PyRequest
r = PyRequest()
r.method = "POST"
assertEqual(r.method, "POST")
"#,
        );
    }

    #[test]
    fn pr03_set_method_invalid_raises() {
        with_module(
            r#"
from roxy import PyRequest
r = PyRequest()
threw = False
try:
    r.method = " NOPE "  # clearly invalid
except Exception:
    threw = True
assert threw, "setting invalid HTTP method should raise"
"#,
        );
    }

    #[test]
    fn pr04_set_version_valid_roundtrip() {
        with_module(
            r#"
from roxy import PyRequest
r = PyRequest()
r.version = "HTTP/1.1"
assertEqual(r.version, "HTTP/1.1")
r.version = "HTTP/2.0"
assertEqual(r.version, "HTTP/2.0")
"#,
        );
    }

    #[test]
    fn pr05_set_version_invalid_raises() {
        with_module(
            r#"
from roxy import PyRequest
r = PyRequest()
threw = False
try:
    r.version = "HTTP/9.9"
except Exception:
    threw = True
assert threw, "invalid HTTP version must raise"
"#,
        );
    }
}
