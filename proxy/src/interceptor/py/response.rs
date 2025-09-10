use std::sync::{Arc, Mutex};

use pyo3::{Py, PyResult, exceptions::PyTypeError, pyclass, pymethods};
use roxy_shared::version::HttpVersion;

use crate::{
    flow::InterceptedResponse,
    interceptor::py::{body::PyBody, headers::PyHeaders},
};

#[pyclass]
pub(crate) struct PyResponse {
    pub(crate) inner: Arc<Mutex<InterceptedResponse>>,
    #[pyo3(get)]
    pub(crate) body: Py<PyBody>,
    #[pyo3(get)]
    pub(crate) headers: Py<PyHeaders>,
    #[pyo3(get)]
    pub(crate) trailers: Py<PyHeaders>,
}

impl PyResponse {
    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, InterceptedResponse>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyResponse {
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
        Ok(format!("{:?}", res.version))
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
