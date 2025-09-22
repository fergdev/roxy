pub mod body;
mod constants;
pub mod engine;
mod extension;
mod flow;
mod headers;
mod notify;
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
    use super::extension::Extension;

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

    #[pymodule_export]
    use super::constants::PyMethod;

    #[pymodule_export]
    use super::constants::PyProtocol;

    #[pymodule_export]
    use super::constants::PyVersion;

    #[pymodule_export]
    use super::notify::notify;
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
#[allow(clippy::panic, clippy::expect_used)]
pub(crate) fn with_module(code: &str) {
    use crate::init_test_logging;

    init_test_logging();
    init_python();
    if let Err(e) = Python::attach(|py| -> pyo3::PyResult<()> {
        py.import("roxy")?;
        py.run(
            &std::ffi::CString::new(
                "#
def assertEqual(a, b, msg=None):
    if a != b:
        if msg is None:
            msg = f\"Expected '{a}' to equal '{b}'\"
        raise AssertionError(msg)


def assertTrue(a, msg=None):
    if not a:
        if msg is None:
            msg = f\"Expected '{a}' to be 'true'\"
        raise AssertionError(msg)

def assertFalse(a, msg=None):
    if a:
        if msg is None:
            msg = f\"Expected '{a}' to be 'false'\"
        raise AssertionError(msg)
#",
            )
            .expect("Invalid cstring"),
            None,
            None,
        )?;

        py.run(
            &std::ffi::CString::new(code).expect("Invalid cstring"),
            None,
            None,
        )?;
        Ok(())
    }) {
        error!("Python error: {e:#?}");
        panic!("Python code failed");
    }
}
