use http::{HeaderMap, HeaderName, HeaderValue};
use pyo3::{exceptions::PyTypeError, prelude::*};
use std::sync::{Arc, Mutex, MutexGuard};

fn to_header_name(name: &str) -> PyResult<HeaderName> {
    HeaderName::from_bytes(name.as_bytes())
        .map_err(|e| PyTypeError::new_err(format!("Invalid header name {e}")))
}

fn to_header_value(val: &str) -> PyResult<HeaderValue> {
    HeaderValue::from_str(val)
        .map_err(|e| PyTypeError::new_err(format!("Invalid header value {e}")))
}

pub(crate) type HeaderList = Arc<Mutex<HeaderMap>>;

#[pyclass]
pub(crate) struct PyHeaders {
    pub(crate) inner: HeaderList,
}

impl PyHeaders {
    fn lock(&self) -> PyResult<MutexGuard<'_, HeaderMap>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyHeaders {
    fn append(&self, _py: Python<'_>, name: &str, value: &str) -> PyResult<()> {
        let name = to_header_name(name)?;
        let value = to_header_value(value)?;
        let mut g = self.lock()?;
        g.append(name, value);
        Ok(())
    }

    fn set(&self, _py: Python<'_>, name: &str, value: &str) -> PyResult<()> {
        let name = to_header_name(name)?;
        let value = to_header_value(value)?;
        let mut g = self.lock()?;
        g.insert(name, value);
        Ok(())
    }

    fn get(&self, _py: Python<'_>, name: &str) -> PyResult<Option<String>> {
        let name = to_header_name(name)?;
        let g = self.lock()?;
        Ok(g.get(&name).map(|v| v.to_str().unwrap_or("").to_string()))
    }
}
