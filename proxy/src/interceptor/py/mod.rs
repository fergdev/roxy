pub mod body;
pub mod engine;
mod flow;
mod headers;
mod query;
mod request;
mod response;
mod url;

use std::sync::Once;

use pyo3::{Python, pymodule};
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
