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

#[derive(Debug, Clone)]
#[pyclass]
pub(crate) struct PyHeaders {
    pub(crate) inner: HeaderList,
}

impl Default for PyHeaders {
    fn default() -> Self {
        PyHeaders {
            inner: Arc::new(Mutex::new(HeaderMap::new())),
        }
    }
}

impl PyHeaders {
    pub(crate) fn from_headers(headers: HeaderMap) -> PyHeaders {
        PyHeaders {
            inner: Arc::new(Mutex::new(headers)),
        }
    }

    fn lock(&self) -> PyResult<MutexGuard<'_, HeaderMap>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }
}

#[pymethods]
impl PyHeaders {
    #[new]
    fn new_py() -> Self {
        Self::default()
    }

    fn append(&self, _py: Python<'_>, name: &str, value: &str) -> PyResult<()> {
        let name = to_header_name(name)?;
        let value = to_header_value(value)?;
        let mut g = self.lock()?;
        g.append(name, value);
        Ok(())
    }

    fn set(&self, name: &str, value: &str) -> PyResult<()> {
        let name = to_header_name(name)?;
        let value = to_header_value(value)?;
        let mut g = self.lock()?;
        g.insert(name, value);
        Ok(())
    }

    fn __setitem__(&mut self, key: &str, value: &Bound<PyAny>) -> PyResult<()> {
        if value.is_none() {
            self.delete(key)
        } else {
            let s = value.extract::<String>()?;
            self.set(key, &s)
        }
    }

    fn delete(&self, name: &str) -> PyResult<()> {
        let name = to_header_name(name)?;
        let mut g = self.lock()?;
        g.remove(name);
        Ok(())
    }

    fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        let name = to_header_name(key)?;
        let mut g = self.lock()?;
        g.remove(name);
        Ok(())
    }

    fn get(&self, _py: Python<'_>, name: &str) -> PyResult<Option<String>> {
        let name = to_header_name(name)?;
        let g = self.lock()?;
        Ok(g.get(&name).map(|v| v.to_str().unwrap_or("").to_string()))
    }

    fn has(&self, _py: Python<'_>, name: &str) -> PyResult<bool> {
        let name = to_header_name(name)?;
        let g = self.lock()?;
        Ok(g.contains_key(name))
    }

    fn clear(&self) -> PyResult<()> {
        let mut guard = self.lock()?;
        guard.clear();
        Ok(())
    }

    fn __str__(&self) -> PyResult<String> {
        let guard = self.lock()?;
        Ok(format!("{:?}", *guard))
    }
    fn __len__(&self) -> PyResult<usize> {
        let g = self.lock()?;
        Ok(g.len())
    }
    fn __repr__(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(format!("Headers(len={:?}, values={:?})", g.len(), g))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {

    use crate::interceptor::py::with_module;
    #[test]
    fn pyheaders_append_and_get() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
h.append("x-test", "v1")
h.append("x-test-2", "v2")
assertEqual(h.get("x-test"), "v1")
assertEqual(h.get("x-test-2"), "v2")
assert h.get("missing") is None
"#,
        );
    }
    #[test]
    fn pyheaders_is_empty() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
h.append("x-test", "v1")
h.append("x-test-2", "v2")
assertFalse(not h)
h.clear()
assertTrue(not h)
"#,
        );
    }

    #[test]
    fn pyheaders_set_overwrites_previous_values() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
h.append("x", "a")
h.set("x", "b")      # must overwrite
assertEqual(h.get("x"), "b")
"#,
        );
    }

    #[test]
    fn pyheaders_delete_and_has() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
h.set("x-del", "z")
assert h.has("x-del") is True
h.delete("x-del")
assert h.has("x-del") is False
assert h.get("x-del") is None
"#,
        );
    }

    #[test]
    fn pyheaders_clear_empties_all() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
h.set("a", "1")
h.set("b", "2")
h.clear()
assert h.get("a") is None
assert h.get("b") is None
assert h.has("a") is False
assert h.has("b") is False
"#,
        );
    }

    #[test]
    fn pyheaders_invalid_header_name_errors() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
threw = False
try:
    h.set("Bad Name", "x")     # space not allowed in header name
except Exception as e:
    threw = True
assert threw, "expected invalid header name to raise"
"#,
        );
    }

    #[test]
    fn pyheaders_invalid_header_value_errors() {
        with_module(
            r#"
from roxy import PyHeaders
h = PyHeaders()
threw = False
try:
    h.set("X-Thing", "line1\r\nline2")   # CRLF forbidden
except Exception as e:
    threw = True
assert threw, "expected invalid header value to raise"
"#,
        );
    }
}
