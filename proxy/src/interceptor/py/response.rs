use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use pyo3::{
    Bound, PyAny, PyResult, exceptions::PyTypeError, pyclass, pymethods, types::PyAnyMethods,
};
use roxy_shared::version::HttpVersion;
use tracing::info;

use crate::{
    flow::InterceptedResponse,
    interceptor::py::{
        body::PyBody,
        constants::{PyStatus, PyVersion},
        headers::PyHeaders,
    },
};

#[pyclass(from_py_object, name = "Response")]
#[derive(Debug, Clone)]
pub(crate) struct PyResponse {
    pub(crate) status: Arc<Mutex<PyStatus>>,
    pub(crate) version: Arc<Mutex<PyVersion>>,
    #[pyo3(get)]
    pub(crate) body: PyBody,
    #[pyo3(get)]
    pub(crate) headers: PyHeaders,
    #[pyo3(get)]
    pub(crate) trailers: PyHeaders,
}

impl Default for PyResponse {
    fn default() -> Self {
        Self {
            version: Arc::new(Mutex::new(PyVersion::default())),
            status: Arc::new(Mutex::new(PyStatus::default())),
            body: PyBody::default(),
            headers: PyHeaders::default(),
            trailers: PyHeaders::default(),
        }
    }
}

impl PyResponse {
    pub(crate) fn from_resp(resp: &InterceptedResponse) -> Self {
        Self {
            version: Arc::new(Mutex::new(PyVersion::from(&resp.version))),
            status: Arc::new(Mutex::new(PyStatus::from(resp.status))),
            body: PyBody::new(resp.body.clone()),
            headers: PyHeaders::from_headers(resp.headers.clone()),
            trailers: PyHeaders::from_headers(resp.trailers.clone().unwrap_or_default()),
        }
    }
}

#[pymethods]
impl PyResponse {
    #[new]
    fn new_py() -> Self {
        Self::default()
    }

    #[getter]
    fn status(&self) -> PyResult<PyStatus> {
        let res = self
            .status
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        Ok(res.clone())
    }

    #[setter]
    fn set_status(&self, value: Bound<PyAny>) -> PyResult<()> {
        let mut g = self
            .status
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        if let Ok(version) = value.extract::<PyStatus>() {
            *g = version.clone();
            return Ok(());
        }

        if let Ok(s) = value.extract::<u16>() {
            *g = PyStatus::try_from(s)?;
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "invalid http status {value:?}"
        )))
    }

    #[getter]
    fn version(&self) -> PyResult<PyVersion> {
        let res = self
            .version
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))?;
        Ok(res.clone())
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
        Ok(format!("Response({:?})", self))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {

    use crate::interceptor::py::with_module;

    #[test]
    fn pyresponse_constructor_defaults() {
        with_module(
            r#"
from roxy import Response
r = Response()
# We don't assert a specific default status/version; just ensure they are readable.
_ = r.status
_ = r.version
"#,
        );
    }

    #[test]
    fn pyresponse_status_get_set_valid() {
        with_module(
            r#"
from roxy import Response
r = Response()
r.status = 204
assertEqual(r.status, 204)
r.status = 418
assertEqual(r.status, 418)
"#,
        );
    }

    #[test]
    fn pyresponse_status_set_invalid_raises() {
        with_module(
            r#"
from roxy import Response
r = Response()
threw = False
try:
    r.status = 9999
except Exception:
    threw = True
assert threw, "setting invalid HTTP status should raise"
"#,
        );
    }

    #[test]
    fn pyresponse_version_get_set_valid() {
        with_module(
            r#"
from roxy import Response
r = Response()
r.version = "HTTP/1.1"
assertEqual(r.version, "HTTP/1.1")
r.version = "HTTP/2.0"
assertEqual(r.version, "HTTP/2.0")
r.version = "HTTP/3.0"
assertEqual(r.version, "HTTP/3.0")
"#,
        );
    }

    #[test]
    fn pyresponse_version_set_invalid_raises() {
        with_module(
            r#"
from roxy import Response
r = Response()
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
