use std::sync::{Arc, Mutex};

use http::uri::Authority;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use roxy_shared::uri::RUri;

use crate::interceptor::py::query::PyURLSearchParams;

#[pyclass(name = "URL")]
pub struct PyUrl {
    pub(crate) uri: Arc<Mutex<RUri>>,
}

impl PyUrl {
    pub fn from_ruri(r: RUri) -> Self {
        Self {
            uri: Arc::new(Mutex::new(r)),
        }
    }
    pub fn from_arc(uri: Arc<Mutex<RUri>>) -> Self {
        Self { uri }
    }

    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, RUri>> {
        self.uri
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }

    fn href_get(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.to_string())
    }

    fn href_set(&self, href: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        let parsed: http::Uri = href
            .parse()
            .map_err(|e| PyTypeError::new_err(format!("{e}")))?;
        *g = RUri::new(parsed);
        Ok(())
    }

    fn with_parts_mut<F>(&self, f: F) -> PyResult<()>
    where
        F: FnOnce(&mut http::uri::Parts) -> PyResult<()>,
    {
        let mut g = self.lock()?;
        let mut parts = g.inner.clone().into_parts();
        f(&mut parts)?;
        *g = RUri::new(
            http::Uri::from_parts(parts).map_err(|e| PyTypeError::new_err(e.to_string()))?,
        );
        Ok(())
    }
}

#[pymethods]
impl PyUrl {
    #[new]
    fn new(href: &str) -> PyResult<Self> {
        let parsed: http::Uri = href
            .parse()
            .map_err(|e| PyTypeError::new_err(format!("{e}")))?;
        Ok(Self::from_ruri(RUri::new(parsed)))
    }

    #[getter]
    fn href(&self) -> PyResult<String> {
        self.href_get()
    }
    #[setter]
    fn set_href(&self, href: &str) -> PyResult<()> {
        self.href_set(href)
    }

    #[getter]
    fn scheme(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(g.inner.scheme_str().map(|s| s.to_string()))
    }
    #[setter]
    fn set_scheme(&self, proto: &str) -> PyResult<()> {
        let p = proto.strip_suffix(':').unwrap_or(proto);
        self.with_parts_mut(|parts| {
            parts.scheme = Some(
                p.parse()
                    .map_err(|e| PyTypeError::new_err(format!("{e}")))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn username(&self) -> PyResult<String> {
        let g = self.lock()?;
        let auth = g.inner.authority().map(|a| a.as_str()).unwrap_or("");
        let userinfo = auth.split('@').next().unwrap_or("");
        if userinfo.contains(':') || auth.contains('@') {
            Ok(userinfo.split(':').next().unwrap_or("").to_string())
        } else {
            Ok("".to_string())
        }
    }
    #[setter]
    fn set_username(&self, user: &str) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            let (host, port) = parts
                .authority
                .as_ref()
                .map(|a| (a.host().to_string(), a.port_u16()))
                .unwrap_or((String::new(), None));
            let mut auth = String::new();
            if !user.is_empty() {
                auth.push_str(user);
                auth.push('@');
            }
            auth.push_str(&host);
            if let Some(p) = port {
                auth.push(':');
                auth.push_str(&p.to_string());
            }
            parts.authority = Some(
                Authority::from_maybe_shared(auth)
                    .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn password(&self) -> PyResult<String> {
        let g = self.lock()?;
        let auth = g.inner.authority().map(|a| a.as_str()).unwrap_or("");
        let (userinfo, _) = if let Some(pos) = auth.rfind('@') {
            (&auth[..pos], &auth[pos + 1..])
        } else {
            ("", auth)
        };
        Ok(userinfo.split(':').nth(1).unwrap_or("").to_string())
    }

    #[setter]
    fn set_password(&self, pass: &str) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            let (mut user, host, port) = {
                let (h, p) = parts
                    .authority
                    .as_ref()
                    .map(|a| (a.host().to_string(), a.port_u16()))
                    .unwrap_or((String::new(), None));
                (String::new(), h, p)
            };
            let mut auth = String::new();
            if !user.is_empty() || !pass.is_empty() {
                if user.is_empty() {
                    user = "".into();
                }
                auth.push_str(&user);
                auth.push(':');
                auth.push_str(pass);
                auth.push('@');
            }
            auth.push_str(&host);
            if let Some(p) = port {
                auth.push(':');
                auth.push_str(&p.to_string());
            }
            parts.authority = Some(
                Authority::from_maybe_shared(auth)
                    .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn hostname(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(g.inner.authority().map(|a| a.host().to_string()))
    }
    #[setter]
    fn set_hostname(&self, host: &str) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            let port = parts.authority.as_ref().and_then(|a| a.port_u16());
            let auth = if let Some(p) = port {
                format!("{host}:{p}")
            } else {
                host.to_string()
            };
            parts.authority = Some(
                Authority::from_maybe_shared(auth)
                    .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn host(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(g.inner.authority().map(|a| a.as_str().to_string()))
    }
    #[setter]
    fn set_host(&self, host_port: &str) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            parts.authority = Some(
                Authority::try_from(host_port).map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn port(&self) -> PyResult<Option<u16>> {
        let g = self.lock()?;
        Ok(g.inner.port_u16())
    }
    #[setter]
    fn set_port(&self, port: u16) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            let host = parts
                .authority
                .as_ref()
                .map(|a| a.host().to_string())
                .unwrap_or_default();
            let auth = format!("{host}:{port}");
            parts.authority = Some(
                Authority::from_maybe_shared(auth)
                    .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn path(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.inner.path().to_string())
    }
    #[setter]
    fn set_path(&self, path: &str) -> PyResult<()> {
        self.with_parts_mut(|parts| {
            let q = parts
                .path_and_query
                .as_ref()
                .and_then(|pq| pq.query())
                .unwrap_or("");
            let pq = if q.is_empty() {
                path.to_string()
            } else {
                format!("{path}?{q}")
            };
            parts.path_and_query = Some(
                http::uri::PathAndQuery::from_maybe_shared(pq)
                    .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn search(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.inner.query().map(|q| format!("?{q}")).unwrap_or_default())
    }

    #[setter]
    fn set_search(&self, search: &str) -> PyResult<()> {
        let s = search.strip_prefix('?').unwrap_or(search);
        self.with_parts_mut(|parts| {
            let path = parts
                .path_and_query
                .as_ref()
                .map(|pq| pq.path().to_owned())
                .unwrap_or("/".to_string());
            parts.path_and_query = Some(
                if s.is_empty() {
                    http::uri::PathAndQuery::from_maybe_shared(path)
                } else {
                    http::uri::PathAndQuery::from_maybe_shared(format!("{path}?{s}"))
                }
                .map_err(|e| PyTypeError::new_err(e.to_string()))?,
            );
            Ok(())
        })
    }

    #[getter]
    fn origin(&self) -> PyResult<String> {
        let g = self.lock()?;
        let scheme = g.inner.scheme_str().unwrap_or("");
        let host = g.inner.authority().map(|a| a.host()).unwrap_or("");
        let port = g.inner.port_u16();
        Ok(match port {
            Some(p) => format!("{scheme}://{host}:{p}"),
            None => format!("{scheme}://{host}"),
        })
    }

    #[getter]
    fn search_params(&self, py: Python<'_>) -> PyResult<Py<PyURLSearchParams>> {
        Py::new(py, PyURLSearchParams::new(self.uri.clone()))
    }

    fn __str__(&self) -> PyResult<String> {
        self.href_get()
    }
}
