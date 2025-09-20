pub mod body;
pub mod engine;
mod flow;
mod headers;
mod query;
mod request;
mod response;
mod url;
mod writer;

use std::sync::Once;

use pyo3::{PyResult, Python, pymodule, types::PyAnyMethods};
use tracing::error;

use crate::interceptor::py::writer::{WriterStdErr, WriterStdOut};
#[pymodule]
mod roxy {

    #[pymodule_export]
    use super::body::PyBody;

    #[pymodule_export]
    use super::flow::PyFlow;

    #[pymodule_export]
    use super::headers::PyHeaders;

    #[pymodule_export]
    use super::url::PyUrl;

    #[pymodule_export]
    use super::query::PyURLSearchParams;

    #[pymodule_export]
    use super::response::PyResponse;

    #[pymodule_export]
    use super::request::PyRequest;
}

static INIT: Once = Once::new();

pub(crate) fn init_python() {
    INIT.call_once(|| {
        pyo3::append_to_inittab!(roxy);
        Python::initialize();
        if let Err(err) = Python::attach::<_, PyResult<()>>(|py| {
            let sys = py.import("sys")?;
            let out = pyo3::Py::new(py, WriterStdOut)?;
            sys.setattr("stdout", out.clone_ref(py))?;
            let err = pyo3::Py::new(py, WriterStdErr)?;
            sys.setattr("stderr", err)?;
            Ok(())
        }) {
            error!("Error setting writer {err}");
        }
    });
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn with_module(code: &str) {
    init_python();
    Python::attach(|py| -> pyo3::PyResult<()> {
        py.import("roxy")?;
        py.run(
            &std::ffi::CString::new(code).expect("Invalid cstring"),
            None,
            None,
        )?;
        Ok(())
    })
    .expect("python ok");
}
