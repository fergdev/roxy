use std::{
    ops::Deref,
    str::FromStr,
    sync::{Arc, Mutex},
};

use http::Method;
use pyo3::{
    Bound, PyAny, PyResult, exceptions::PyTypeError, pyclass, pymethods, types::PyAnyMethods,
};
use roxy_shared::version::HttpVersion;
use tracing::{error, info};

use crate::{
    flow::InterceptedRequest,
    interceptor::py::{
        body::PyBody,
        constants::{PyMethod, PyVersion},
        headers::PyHeaders,
        url::PyUrl,
    },
};

#[derive(Debug, Clone)]
#[pyclass(from_py_object, name = "Request")]
pub(crate) struct PyRequest {
    pub(crate) method: Arc<Mutex<PyMethod>>,
    pub(crate) version: Arc<Mutex<PyVersion>>,
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
            method: Arc::new(Mutex::new(PyMethod::default())),
            version: Arc::new(Mutex::new(PyVersion::default())),
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
            method: Arc::new(Mutex::new(PyMethod::from(&req.method))),
            version: Arc::new(Mutex::new(PyVersion::from(&req.version))),
            body: PyBody::new(req.body.clone()),
            url: PyUrl::from_ruri(req.uri.clone()),
            headers: PyHeaders::from_headers(req.headers.clone()),
            trailers: PyHeaders::from_headers(req.trailers.clone().unwrap_or_default()),
        }
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
        let mut g = self
            .method
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        if let Ok(method) = py_val.extract::<PyMethod>() {
            *g = method.clone();
            return Ok(());
        }

        if let Ok(s) = py_val.extract::<String>() {
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
    fn version(&self) -> PyResult<PyVersion> {
        let g = self
            .version
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        Ok(g.clone())
    }
    #[setter]
    fn set_version(&self, py_val: Bound<PyAny>) -> PyResult<()> {
        info!("set version");
        let mut g = self
            .version
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        if let Ok(version) = py_val.extract::<PyVersion>() {
            info!("set py version");
            *g = version.clone();
            return Ok(());
        }

        if let Ok(s) = py_val.extract::<String>() {
            info!("set py string");
            *g = PyVersion::from(
                &HttpVersion::from_str(&s)
                    .map_err(|_| PyTypeError::new_err("Failed to convert version"))?,
            );
            return Ok(());
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "method must be Method enum or string",
        ))
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{self:?}"))
    }
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("Request({:?})", self))
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
from roxy import Request
# Construct with defaults; just ensure it doesn't throw and exposes attributes
r = Request()
assert hasattr(r, "method")
assert hasattr(r, "version")
"#,
        );
    }

    #[test]
    fn pr02_set_method_valid() {
        with_module(
            r#"
from roxy import Request
r = Request()
r.method = "POST"
assertEqual(r.method, "POST")
"#,
        );
    }

    #[test]
    fn pr03_set_method_invalid_raises() {
        with_module(
            r#"
from roxy import Request
r = Request()
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
from roxy import Request
r = Request()
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
from roxy import Request
r = Request()
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
