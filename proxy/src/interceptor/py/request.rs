use std::sync::{Arc, Mutex, MutexGuard};

use http::Method;
use pyo3::{Py, PyResult, exceptions::PyTypeError, pyclass, pymethods};
use roxy_shared::version::HttpVersion;

use crate::{
    flow::InterceptedRequest,
    interceptor::py::{body::PyBody, headers::PyHeaders, url::PyUrl},
};

#[pyclass]
pub(crate) struct PyRequest {
    pub(crate) inner: Arc<Mutex<InterceptedRequest>>,
    #[pyo3(get)]
    pub(crate) body: Py<PyBody>,
    #[pyo3(get)]
    pub(crate) url: Py<PyUrl>,
    #[pyo3(get)]
    pub(crate) headers: Py<PyHeaders>,
    #[pyo3(get)]
    pub(crate) trailers: Py<PyHeaders>,
}

impl PyRequest {
    fn lock(&self) -> PyResult<MutexGuard<'_, InterceptedRequest>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyRequest {
    #[getter]
    fn method(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.method.to_string())
    }
    #[setter]
    fn set_method(&self, value: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        g.method = Method::try_from(value).map_err(|e| PyTypeError::new_err(e.to_string()))?;
        Ok(())
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
}
