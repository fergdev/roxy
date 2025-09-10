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
