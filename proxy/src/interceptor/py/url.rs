use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use roxy_shared::uri::RUri;
use url::Url;

use crate::interceptor::py::constants::PyProtocol;
use crate::interceptor::py::query::PyURLSearchParams;
use crate::interceptor::util::set_url_authority;

#[derive(Debug, Clone)]
#[pyclass(from_py_object, name = "URL")]
pub(crate) struct PyUrl {
    pub(crate) inner: Arc<Mutex<Url>>,
}

impl Default for PyUrl {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::from_str("http://localhost/").expect("default URL is valid")
    }
}

impl PyUrl {
    pub fn from_str(s: &str) -> PyResult<Self> {
        let url = Url::parse(s).map_err(|e| PyTypeError::new_err(format!("{e}")))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(url)),
        })
    }
    #[allow(clippy::expect_used)]
    pub fn from_ruri(r: RUri) -> Self {
        Self::from_str(r.to_string().as_ref()).expect("RUri is always valid URL")
    }

    fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, Url>> {
        self.inner
            .lock()
            .map_err(|e| PyTypeError::new_err(format!("lock poisoned: {e}")))
    }

    fn href_get(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.to_string())
    }

    fn href_set(&self, href: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        *g = Url::parse(href).map_err(|e| PyTypeError::new_err(format!("{e}")))?;
        Ok(())
    }
}

#[pymethods]
impl PyUrl {
    #[new]
    fn new(href: &str) -> PyResult<Self> {
        Self::from_str(href)
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
    fn protocol(&self) -> PyResult<PyProtocol> {
        let g = self.lock()?;
        Ok(PyProtocol::from(g.scheme()))
    }
    #[setter]
    fn set_protocol(&self, py_val: Bound<PyAny>) -> PyResult<()> {
        let mut g = self.lock()?;
        if let Ok(proto) = py_val.extract::<PyProtocol>() {
            url::quirks::set_protocol(&mut g, &proto.to_string())
                .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))?;
            return Ok(());
        }

        if let Ok(s) = py_val.extract::<String>() {
            url::quirks::set_protocol(&mut g, &s)
                .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))?;
            return Ok(());
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "method must be Method enum or string",
        ))
    }

    #[getter]
    fn username(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(match g.username() {
            "" => None,
            u => Some(u.to_owned()),
        })
    }

    #[setter]
    fn set_username(&self, user: &str) -> PyResult<()> {
        let mut u = self.lock()?;
        u.set_username(user)
            .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn password(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(g.password().map(|p| p.to_owned()))
    }

    #[setter]
    fn set_password(&self, pass: &str) -> PyResult<()> {
        let mut u = self.lock()?;
        u.set_password(Some(pass))
            .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn hostname(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(Some(url::quirks::hostname(&g).to_owned()))
    }

    #[setter]
    fn set_hostname(&self, hostname: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        url::quirks::set_hostname(&mut g, hostname)
            .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn host(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(url::quirks::host(&g).to_owned())
    }
    #[setter]
    fn set_host(&self, host_port: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        url::quirks::set_host(&mut g, host_port)
            .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn port(&self) -> PyResult<Option<u16>> {
        let g = self.lock()?;
        Ok(g.port())
    }
    #[setter]
    fn set_port(&self, port: u16) -> PyResult<()> {
        let mut g = self.lock()?;
        url::quirks::set_port(&mut g, &format!("{port}"))
            .map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn path(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.path().to_owned())
    }
    #[setter]
    fn set_path(&self, path: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        url::quirks::set_pathname(&mut g, path);
        Ok(())
    }

    #[getter]
    fn authority(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(g.authority().to_owned())
    }
    #[setter]
    fn set_authority(&self, authority: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        set_url_authority(&mut g, authority).map_err(|e| PyTypeError::new_err(format!("{e:#?}")))
    }

    #[getter]
    fn search(&self) -> PyResult<Option<String>> {
        let g = self.lock()?;
        Ok(g.query().map(|q| q.to_owned()))
    }

    #[setter]
    fn set_search(&self, search: &str) -> PyResult<()> {
        let mut g = self.lock()?;
        url::quirks::set_search(&mut g, search);
        Ok(())
    }

    #[getter]
    fn search_params(&self, py: Python<'_>) -> PyResult<Py<PyURLSearchParams>> {
        Py::new(py, PyURLSearchParams::new(self.inner.clone()))
    }

    fn __str__(&self) -> PyResult<String> {
        self.href_get()
    }
    fn __repr__(&self) -> PyResult<String> {
        let g = self.lock()?;
        Ok(format!("URL({:?})", g))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::py::with_module;

    #[test]
    fn pyurl_constructor_roundtrip() {
        with_module(
            r#"
from roxy import URL
u = URL("http://example.com:8080/path?x=1")
assertEqual(str(u), "http://example.com:8080/path?x=1")
"#,
        );
    }

    #[test]
    fn pyurl_href_get_set_valid() {
        with_module(
            r#"
from roxy import URL
u = URL("http://a/")
assertEqual(u.href, "http://a/")
u.href = "https://example.org/zzz?q=1"
assertEqual(u.href, "https://example.org/zzz?q=1")
assertEqual(str(u), "https://example.org/zzz?q=1")
"#,
        );
    }
    #[test]
    fn pyurl_protocol_get_set_str() {
        with_module(
            r#"
from roxy import URL
u = URL("http://example.com/")
assertEqual(u.protocol, "HTTP")
assertEqual(u.protocol, "http")
u.protocol = "https"
assertEqual(u.protocol, "https")
assertEqual(u.protocol, "HTTPS")
assert u.href.startswith("https://")
"#,
        );
    }

    #[test]
    fn pyurl_protocol_get_set() {
        with_module(
            r#"
from roxy import URL, Protocol
u = URL("http://example.com/")
assertEqual(u.protocol, Protocol.HTTP)
u.protocol = Protocol.HTTPS
assertEqual(u.protocol, Protocol.HTTPS)
assert u.href.startswith("https://")
"#,
        );
    }

    #[test]
    fn pyurl_username_password_get_set() {
        with_module(
            r#"
from roxy import URL
u = URL("http://example.com/")
assertEqual(u.username, None)
assertEqual(u.password, None)
u.username = "user"
assertEqual(u.username, "user")
u.password = "pass"
assertEqual(u.password, "pass")
u.username = "newuser"
assertEqual(u.username, "newuser")
assertEqual(u.password, "pass")
"#,
        );
    }

    #[test]
    fn pyurl_path_get_set_preserves_query() {
        with_module(
            r#"
from roxy import URL
u = URL("http://example.com/a/b?x=1")
assertEqual(u.path, "/a/b")
u.path = "/z"
assertEqual(u.path, "/z")
assertEqual(u.search, "x=1")
assertTrue(str(u).startswith("http://example.com/z?x=1"))
"#,
        );
    }

    #[test]
    fn pyurl_search_get_set_and_params() {
        with_module(
            r#"
from roxy import URL
u = URL("http://example.com/")
print(str(u.search))
assertEqual(u.search, None)
u.search = "a=1&b=2"
assertEqual(u.search, "a=1&b=2")

sp = u.search_params
assertEqual(sp.get("a"), "1")
sp.set("a", "9")
assertEqual(sp.get("a"), "9")
sp.append("a", "10")
vals = sp.get_all("a")
assertEqual(vals, ["9", "10"])
sp.delete("b")
assertTrue(not sp.has("b"))
assertTrue(str(u).endswith("?a=9&a=10"))
"#,
        );
    }

    #[test]
    fn pyurl_href_set_invalid_errors() {
        with_module(
            r#"
from roxy import URL
u = URL("http://ok/")
threw = False
try:
    u.href = "http://exa mple.com/"  # space invalid
except Exception:
    threw = True
assert threw, "invalid href should raise"
"#,
        );
    }
}
