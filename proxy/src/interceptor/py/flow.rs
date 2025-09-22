use pyo3::{Py, PyResult, Python, pyclass, pymethods};

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::py::{request::PyRequest, response::PyResponse},
};

#[derive(Debug, Clone)]
#[pyclass(name = "Flow")]
#[derive(Default)]
pub(crate) struct PyFlow {
    #[pyo3(get)]
    pub(crate) request: PyRequest,
    #[pyo3(get)]
    pub(crate) response: PyResponse,
}

impl PyFlow {
    pub(crate) fn from_data<'py>(
        py: Python<'py>,
        req: &InterceptedRequest,
        resp_opt: &Option<InterceptedResponse>,
    ) -> PyResult<Py<Self>> {
        let resp = resp_opt
            .as_ref()
            .cloned()
            .unwrap_or(InterceptedResponse::default());
        let request = PyRequest::from_req(req);
        let response = PyResponse::from_resp(&resp);
        Py::new(py, PyFlow { request, response })
    }
}

#[pymethods]
impl PyFlow {
    #[new]
    fn new_py() -> Self {
        Self::default()
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{self:?}"))
    }
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Flow(request={:?}, response={:?})",
            self.request, self.response
        ))
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::py::with_module;

    #[test]
    fn pyflow_constructor_defaults() {
        with_module(
            r#"
from roxy import PyFlow
f = PyFlow()
# attributes exist
assert hasattr(f, "request")
assert hasattr(f, "response")
# nested readable basics
_ = f.request.method
_ = f.request.version
_ = f.response.status
_ = f.response.version
"#,
        );
    }

    #[test]
    fn pyflow_request_set_method_and_version() {
        with_module(
            r#"
from roxy import PyFlow
f = PyFlow()
f.request.method = "POST"
assertEqual(f.request.method, "POST")
f.request.version = "HTTP/1.1"
assertEqual(f.request.version, "HTTP/1.1")
"#,
        );
    }

    #[test]
    fn pyflow_response_set_status_and_version() {
        with_module(
            r#"
from roxy import PyFlow
f = PyFlow()
f.response.status = 201
assertEqual(f.response.status, 201)
f.response.version = "HTTP/2.0"
assertEqual(f.response.version, "HTTP/2.0")
"#,
        );
    }

    #[test]
    fn pyflow_body_roundtrip_on_response() {
        with_module(
            r#"
from roxy import PyFlow
f = PyFlow()
assert not f.response.body
assertEqual(len(f.response.body), 0)

f.response.body.text = "hello"
assertEqual(f.response.body.text, "hello")
assertEqual(len(f.response.body), 5)

raw = f.response.body.raw
assert isinstance(raw, (bytes, bytearray))
assertEqual(raw, b"hello")

f.response.body.raw = b"abc\x00def"
assertEqual(len(f.response.body), 7)
assert isinstance(f.response.body.text, str)
"#,
        );
    }

    #[test]
    fn pyflow_request_headers_and_url_present() {
        with_module(
            r#"
from roxy import PyFlow
f = PyFlow()
# request sub-objects exist and are usable (minimal smoke)
h = f.request.headers
t = f.request.trailers
u = f.request.url
# basic API presence
assert hasattr(h, "set")
assert hasattr(t, "set")
assert hasattr(u, "href")
"#,
        );
    }
}
