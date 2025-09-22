use std::sync::{Arc, Mutex, MutexGuard};

use bytes::Bytes;
use pyo3::{
    Bound, PyResult, Python,
    exceptions::PyTypeError,
    pyclass, pymethods,
    types::{PyBytes, PyBytesMethods},
};

#[pyclass]
#[derive(Debug, Clone)]
pub(crate) struct PyBody {
    pub(crate) inner: Arc<Mutex<Bytes>>,
}

impl Default for PyBody {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Bytes::new())),
        }
    }
}

impl PyBody {
    pub(crate) fn new(data: Bytes) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
        }
    }
    fn lock(&self) -> PyResult<MutexGuard<'_, Bytes>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyBody {
    #[new]
    #[pyo3(signature = (value=None))]
    fn new_py(value: Option<&str>) -> Self {
        let bytes = value.unwrap_or("").as_bytes();
        Self::new(Bytes::copy_from_slice(bytes))
    }
    #[getter]
    fn raw<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let g = self.lock()?;
        Ok(PyBytes::new(py, &g.clone()))
    }

    #[setter]
    fn set_raw(&mut self, value: Bound<PyBytes>) -> PyResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::copy_from_slice(value.as_bytes());
        Ok(())
    }

    #[getter]
    fn text(&self) -> PyResult<String> {
        let g = self.lock()?;
        String::from_utf8(g.to_vec())
            .map_err(|e| PyTypeError::new_err(format!("invalid UTF-8: {e}")))
    }

    #[setter]
    fn set_text(&mut self, value: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::copy_from_slice(value.as_bytes());
        Ok(())
    }

    fn clear(&self) -> PyResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::new();
        Ok(())
    }

    fn __len__(&self) -> PyResult<usize> {
        let g = self.lock()?;
        Ok(g.len())
    }

    fn __str__(&self) -> PyResult<String> {
        self.text()
    }

    fn __repr__(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(format!("PyBody(len={}, preview={:?})", g.len(), &g))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::py::with_module;

    #[test]
    fn constructor() {
        with_module(
            r#"
from roxy import PyBody
b = PyBody()
assertEqual(b.text, "")
assertEqual(b.raw, b"")
assertEqual(len(b), 0)
assertTrue(not b)

b = PyBody("seed")
assertEqual(b.text, "seed")
assertEqual(b.raw, b"seed")
assertEqual(len(b), 4)
assertFalse(not b)
"#,
        );
    }

    #[test]
    fn pybody_text_to_raw_roundtrip() {
        with_module(
            r#"
from roxy import PyBody
b = PyBody()
b.text = "abc\x00def"
assertEqual(len(b), 7)
assert isinstance(b.raw, (bytes, bytearray))
assertEqual(b.raw, b"abc\x00def")
"#,
        );
    }

    #[test]
    fn pybody_raw_to_text_roundtrip() {
        with_module(
            r#"
from roxy import PyBody
b = PyBody("x")
b.raw = b"hi"
assertEqual(b.text, "hi")
assertEqual(len(b), 2)
"#,
        );
    }

    #[test]
    fn pybody_repr_contains_len_and_preview() {
        with_module(
            r#"
from roxy import PyBody
b = PyBody("hi")
r = repr(b)
assert "PyBody" in r and "len=2" in r
"#,
        );
    }
}
