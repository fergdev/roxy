use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use pyo3::{exceptions::PyTypeError, types::PyList};
use url::Url;

#[pyclass(name = "URLSearchParams")]
pub struct PyURLSearchParams {
    uri: Arc<Mutex<Url>>,
}

impl Default for PyURLSearchParams {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self {
            uri: Arc::new(Mutex::new(
                Url::parse("http://localhost/").expect("default URL is valid"),
            )),
        }
    }
}

impl PyURLSearchParams {
    pub fn new(uri: Arc<Mutex<Url>>) -> Self {
        Self { uri }
    }

    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, Url>> {
        self.uri
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }

    fn with_pairs_mut<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&mut Vec<(String, String)>) -> PyResult<R>,
    {
        let mut url = self.lock()?;
        let mut pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let out = f(&mut pairs)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.clear();
            qp.extend_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        }

        Ok(out)
    }
}

#[pymethods]
impl PyURLSearchParams {
    #[new]
    #[pyo3(signature = (value=None))]
    #[allow(clippy::expect_used)]
    fn new_with_str(value: Option<&str>) -> PyResult<Self> {
        let mut url = Url::parse("http://localhost/").expect("default URL is valid");
        if let Some(s) = value {
            url::quirks::set_search(&mut url, s);
        }
        Ok(Self::new(Arc::new(Mutex::new(url))))
    }

    fn set(&self, key: &str, value: &Bound<PyAny>) -> PyResult<()> {
        self.with_pairs_mut(|pairs| {
            pairs.retain(|(k, _)| k != key);
            let s = value.str()?.to_str().unwrap_or("").to_string();
            pairs.push((key.to_string(), s));
            Ok(())
        })
    }

    fn append(&self, key: &str, value: &Bound<PyAny>) -> PyResult<()> {
        let s = value.str()?.to_str().unwrap_or("").to_string();
        self.with_pairs_mut(|pairs| {
            pairs.push((key.to_string(), s));
            Ok(())
        })
    }

    fn delete(&self, key: &str) -> PyResult<()> {
        self.with_pairs_mut(|pairs| {
            pairs.retain(|(k, _)| k != key);
            Ok(())
        })
    }

    fn get(&self, key: &str) -> PyResult<Option<String>> {
        let guard = self.lock()?;
        for (k, v) in guard.query_pairs() {
            if k == key {
                return Ok(Some(v.to_string()));
            }
        }
        Ok(None)
    }

    fn get_all<'py>(&self, py: Python<'py>, key: &str) -> PyResult<Bound<'py, PyList>> {
        let guard = self.lock()?;
        let vals: Vec<String> = guard
            .query_pairs()
            .into_iter()
            .filter_map(|(k, v)| (k == key).then_some(v.to_string()))
            .collect::<Vec<String>>();
        PyList::new(py, vals)
    }

    fn has(&self, key: &str) -> PyResult<bool> {
        let guard = self.lock()?;
        Ok(guard.query_pairs().any(|(k, _)| k == key))
    }

    fn clear(&self) -> PyResult<()> {
        let mut guard = self.lock()?;
        guard.set_query(None);
        Ok(())
    }

    fn sort(&self) -> PyResult<()> {
        self.with_pairs_mut(|pairs| {
            pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            Ok(())
        })
    }

    fn __str__(&self) -> PyResult<String> {
        let guard = self.lock()?;
        Ok(guard.query().unwrap_or("").to_owned())
    }
    fn __len__(&self) -> PyResult<usize> {
        let guard = self.lock()?;
        Ok(guard.query_pairs().count())
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        self.get(key)
    }
    fn __setitem__(&self, key: &str, value: &Bound<PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
    fn __delitem__(&self, key: &str) -> PyResult<()> {
        self.delete(key)
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::py::with_module;

    #[test]
    fn p01_constructor_parses_initial_query() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p1 = P("a=1&b=2")
assertEqual(str(p1), "a=1&b=2")
p2 = P("?x=9&x=10")
assertEqual(p2.get("x"), "9")
assertEqual(p2.get_all("x"), ["9","10"])
"#,
        );
    }
    #[test]
    fn is_empty() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p1 = P("a=1&b=2")
assertFalse(not p1)
p1.clear()
assertTrue(not p1)
"#,
        );
    }

    #[test]
    fn p02_set_append_get_delete_has() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("foo=1&bar=2")
assertEqual(p.get("foo"), "1")
p.set("foo", "9")
assertEqual(p.get("foo"), "9")
p.append("foo", "10")
assertEqual(p.get_all("foo"), ["9","10"])
assert p.has("bar") is True
p.delete("foo")
assert p.get("foo") is None
assert p.has("foo") is False
"#,
        );
    }

    #[test]
    fn p03_clear_and_sort() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("z=3&b=2&a=1&b=1")
p.sort()
assertEqual(str(p), "a=1&b=1&b=2&z=3")
p.clear()
assertEqual(str(p), "")
assert p.has("a") is False
"#,
        );
    }

    #[test]
    fn p04_mapping_dunder_index_set_del() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("x=1")
assertEqual(p["x"], "1")
p["x"] = "42"
assertEqual(p.get("x"), "42")
del p["x"]
assert p.get("x") is None
assertEqual(str(p), "")
"#,
        );
    }

    #[test]
    fn p05_str() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("a=1&b=2")
assertEqual(str(p), "a=1&b=2")
"#,
        );
    }

    #[test]
    fn not_is_empty() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("a=1&b=2")
assertFalse(not p, "Not P on non empty should yield false");
p.clear()
assertTrue(not p, "Not P on empty should yield true");
"#,
        );
    }

    #[test]
    fn p06_value_coercion_via_pyany_str() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P()
p.set("n", 123)
p.append("n", True)
p.append("n", 4.5)
assertEqual(p.get_all("n"), ["123","True","4.5"])
"#,
        );
    }

    #[test]
    fn p07_invalid_constructor_string_errors() {
        with_module(
            r#"
from roxy import URLSearchParams as P
_ = P("a=%GG")
"#,
        );
    }
}
