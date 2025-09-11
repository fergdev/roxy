use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use pyo3::{exceptions::PyTypeError, types::PyList};
use url::form_urlencoded::{Serializer, parse as parse_qs};

use roxy_shared::uri::RUri;

fn read_pairs(uri: &RUri) -> Vec<(String, String)> {
    uri.inner
        .query()
        .map(|q| {
            parse_qs(q.as_bytes())
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
        .unwrap_or_default()
}

fn write_pairs(parts: &mut http::uri::Parts, pairs: &[(String, String)]) -> PyResult<()> {
    let path = parts
        .path_and_query
        .as_ref()
        .map(|pq| pq.path().to_owned())
        .unwrap_or("/".to_string());
    let mut ser = Serializer::new(String::new());
    for (k, v) in pairs {
        ser.append_pair(k, v);
    }
    let qs = ser.finish();
    let pq = if qs.is_empty() {
        http::uri::PathAndQuery::from_maybe_shared(path)
    } else {
        http::uri::PathAndQuery::from_maybe_shared(format!("{path}?{qs}"))
    }
    .map_err(|e| PyTypeError::new_err(e.to_string()))?;
    parts.path_and_query = Some(pq);
    Ok(())
}

#[pyclass(name = "URLSearchParams")]
pub struct PyURLSearchParams {
    uri: Arc<Mutex<RUri>>,
}

impl Default for PyURLSearchParams {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(RUri::default())))
    }
}

impl PyURLSearchParams {
    pub fn new(uri: Arc<Mutex<RUri>>) -> Self {
        Self { uri }
    }

    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, RUri>> {
        self.uri
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }

    fn with_pairs_mut<F, R>(&self, f: F) -> PyResult<R>
    where
        F: FnOnce(&mut Vec<(String, String)>) -> PyResult<R>,
    {
        let mut guard = self.lock()?;
        let mut parts = guard.inner.clone().into_parts();
        let mut pairs = read_pairs(&guard);
        let out = f(&mut pairs)?;
        write_pairs(&mut parts, &pairs)?;
        *guard = RUri::new(
            http::Uri::from_parts(parts).map_err(|e| PyTypeError::new_err(e.to_string()))?,
        );
        Ok(out)
    }
}

#[pymethods]
impl PyURLSearchParams {
    #[new]
    #[pyo3(signature = (value=None))]
    fn new_with_str(value: Option<&str>) -> PyResult<Self> {
        let uri = if let Some(s) = value {
            let s = s.strip_prefix('?').unwrap_or(s);
            let full = format!("http://dummy/?{s}");
            match full.parse() {
                Ok(uri) => RUri::new(uri),
                Err(e) => {
                    return Err(PyTypeError::new_err(format!(
                        "failed to parse URLSearchParams from '{s}': {e}"
                    )));
                }
            }
        } else {
            RUri::default()
        };
        Ok(Self::new(Arc::new(Mutex::new(uri))))
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
        for (k, v) in read_pairs(&guard) {
            if k == key {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    fn get_all<'py>(&self, py: Python<'py>, key: &str) -> PyResult<Bound<'py, PyList>> {
        let guard = self.lock()?;
        let vals: Vec<String> = read_pairs(&guard)
            .into_iter()
            .filter_map(|(k, v)| (k == key).then_some(v))
            .collect();
        PyList::new(py, vals)
    }

    fn has(&self, key: &str) -> PyResult<bool> {
        let guard = self.lock()?;
        Ok(read_pairs(&guard).iter().any(|(k, _)| k == key))
    }

    fn clear(&self) -> PyResult<()> {
        self.with_pairs_mut(|pairs| {
            pairs.clear();
            Ok(())
        })
    }

    fn sort(&self) -> PyResult<()> {
        self.with_pairs_mut(|pairs| {
            pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            Ok(())
        })
    }

    fn to_string(&self) -> PyResult<String> {
        let guard = self.lock()?;
        Ok(guard.inner.query().unwrap_or("").to_string())
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

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
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
assert str(p1) == "a=1&b=2"
p2 = P("?x=9&x=10")
assert p2.get("x") == "9"
assert p2.get_all("x") == ["9","10"]
"#,
        );
    }

    #[test]
    fn p02_set_append_get_delete_has() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("foo=1&bar=2")
assert p.get("foo") == "1"
p.set("foo", "9")
assert p.get("foo") == "9"
p.append("foo", "10")
assert p.get_all("foo") == ["9","10"]
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
assert str(p) == "a=1&b=1&b=2&z=3"
p.clear()
assert str(p) == ""
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
assert p["x"] == "1"
p["x"] = "42"
assert p.get("x") == "42"
del p["x"]
assert p.get("x") is None
assert str(p) == ""
"#,
        );
    }

    #[test]
    fn p05_to_string_and_str_match() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P("a=1&b=2")
assert p.to_string() == "a=1&b=2"
assert str(p) == "a=1&b=2"
"#,
        );
    }

    #[test]
    fn p06_value_coercion_via_pyany_str() {
        with_module(
            r#"
from roxy import URLSearchParams as P
p = P()
p.set("n", 123)          # -> "123"
p.append("n", True)      # -> "True"
p.append("n", 4.5)       # -> "4.5"
assert p.get_all("n") == ["123","True","4.5"]
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
