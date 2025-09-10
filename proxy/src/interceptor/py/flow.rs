use std::sync::{Arc, Mutex};

use pyo3::{Py, PyResult, Python, pyclass};

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::py::{
        body::PyBody,
        headers::{HeaderList, PyHeaders},
        request::PyRequest,
        response::PyResponse,
        url::PyUrl,
    },
};

#[pyclass]
pub(crate) struct PyFlow {
    #[pyo3(get)]
    pub(crate) request: Py<PyRequest>,
    #[pyo3(get)]
    pub(crate) response: Py<PyResponse>,
}

pub(crate) fn build_flow<'py>(
    py: Python<'py>,
    req: &InterceptedRequest,
    resp_opt: &Option<InterceptedResponse>,
) -> PyResult<(Py<PyFlow>, HeaderList, HeaderList)> {
    let req_headers: HeaderList = Arc::new(Mutex::new(req.headers.clone()));
    let req_trailers: HeaderList = Arc::new(Mutex::new(req.trailers.clone().unwrap_or_default()));

    let resp = resp_opt
        .as_ref()
        .cloned()
        .unwrap_or(InterceptedResponse::default());
    let resp_headers: HeaderList = Arc::new(Mutex::new(resp.headers.clone()));
    let resp_trailers: HeaderList = Arc::new(Mutex::new(resp.trailers.clone().unwrap_or_default()));

    let py_req_headers = Py::new(
        py,
        PyHeaders {
            inner: req_headers.clone(),
        },
    )?;
    let py_res_headers = Py::new(
        py,
        PyHeaders {
            inner: resp_headers.clone(),
        },
    )?;

    let py_req = Py::new(
        py,
        PyRequest {
            inner: Arc::new(Mutex::new(req.clone())),
            body: Py::new(py, PyBody::new(req.body.clone()))?,
            url: Py::new(py, PyUrl::from_ruri(req.uri.clone()))?,
            headers: py_req_headers,
            trailers: Py::new(
                py,
                PyHeaders {
                    inner: req_trailers.clone(),
                },
            )?,
        },
    )?;
    let resp_body = Py::new(py, PyBody::new(resp.body.clone()))?;
    let py_res = Py::new(
        py,
        PyResponse {
            inner: Arc::new(Mutex::new(resp)),
            body: resp_body,
            headers: py_res_headers,
            trailers: Py::new(
                py,
                PyHeaders {
                    inner: resp_trailers.clone(),
                },
            )?,
        },
    )?;

    let py_flow = Py::new(
        py,
        PyFlow {
            request: py_req,
            response: py_res,
        },
    )?;

    Ok((py_flow, req_headers, resp_headers))
}
