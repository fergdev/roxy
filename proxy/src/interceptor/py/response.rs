use std::sync::{Arc, Mutex};

use pyo3::{PyResult, exceptions::PyTypeError, pyclass, pymethods};
use roxy_shared::version::HttpVersion;

use crate::{
    flow::InterceptedResponse,
    interceptor::py::{body::PyBody, headers::PyHeaders},
};

#[pyclass]
#[derive(Debug, Clone)]
pub(crate) struct PyResponse {
    pub(crate) inner: Arc<Mutex<InterceptedResponse>>,
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
            inner: Arc::new(Mutex::new(InterceptedResponse::default())),
            body: PyBody::default(),
            headers: PyHeaders::default(),
            trailers: PyHeaders::default(),
        }
    }
}

impl PyResponse {
    pub(crate) fn from_resp(resp: &InterceptedResponse) -> Self {
        Self {
            inner: Arc::new(Mutex::new(resp.clone())),
            body: PyBody::new(resp.body.clone()),
            headers: PyHeaders::from_headers(resp.headers.clone()),
            trailers: PyHeaders::from_headers(resp.trailers.clone().unwrap_or_default()),
        }
    }
    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, InterceptedResponse>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyResponse {
    #[new]
    fn new_py() -> Self {
        Self::default()
    }

    #[getter]
    fn status(&self) -> PyResult<u16> {
        let res = self.lock()?;
        Ok(res.status.as_u16())
    }

    #[setter]
    fn set_status(&self, value: u16) -> PyResult<()> {
        let mut res = self.lock()?;
        res.status =
            http::StatusCode::from_u16(value).map_err(|e| PyTypeError::new_err(e.to_string()))?;
        Ok(())
    }

    #[getter]
    fn version(&self) -> PyResult<String> {
        let res = self.lock()?;
        Ok(res.version.to_string())
    }

    #[setter]
    fn set_version(&self, value: String) -> PyResult<()> {
        let mut res = self.lock()?;
        let value: HttpVersion = value
            .parse()
            .map_err(|_| PyTypeError::new_err(format!("Invalid HTTP version: {value}.")))?;
        res.version = value;
        Ok(())
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
from roxy import PyResponse
r = PyResponse()
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
from roxy import PyResponse
r = PyResponse()
r.status = 204
assert r.status == 204
r.status = 418
assert r.status == 418
"#,
        );
    }

    #[test]
    fn pyresponse_status_set_invalid_raises() {
        with_module(
            r#"
from roxy import PyResponse
r = PyResponse()
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
from roxy import PyResponse
r = PyResponse()
r.version = "HTTP/1.1"
assert r.version == "HTTP/1.1"
r.version = "HTTP/2.0"
assert r.version == "HTTP/2.0"
r.version = "HTTP/3.0"
assert r.version == "HTTP/3.0"
"#,
        );
    }

    #[test]
    fn pyresponse_version_set_invalid_raises() {
        with_module(
            r#"
from roxy import PyResponse
r = PyResponse()
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
