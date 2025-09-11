use bytes::Bytes;
use pyo3::{
    Bound, PyResult, Python,
    exceptions::PyTypeError,
    pyclass, pymethods,
    types::{PyBytes, PyBytesMethods},
};

#[pyclass]
#[derive(Debug, Clone)]
pub(crate) struct PyBody {
    pub(crate) inner: Bytes,
}

impl PyBody {
    pub(crate) fn new(data: Bytes) -> Self {
        Self { inner: data }
    }
}

#[pymethods]
impl PyBody {
    #[new]
    fn new_py(value: Option<&str>) -> Self {
        let bytes = value.unwrap_or("").as_bytes();
        PyBody {
            inner: Bytes::copy_from_slice(bytes),
        }
    }
    #[getter]
    fn raw<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner)
    }

    #[setter]
    fn set_raw(&mut self, value: Bound<PyBytes>) {
        self.inner = Bytes::copy_from_slice(value.as_bytes());
    }

    #[getter]
    fn text(&self) -> PyResult<String> {
        String::from_utf8(self.inner.to_vec())
            .map_err(|e| PyTypeError::new_err(format!("invalid UTF-8: {e}")))
    }

    #[setter]
    fn set_text(&mut self, value: &str) {
        self.inner = Bytes::copy_from_slice(value.as_bytes());
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "PyBody(len={}, preview={:?})",
            self.inner.len(),
            &self.inner
        )
    }
}

// #[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// #[cfg(test)]
// mod tests {
//     use std::ffi::CString;
//
//     use super::*;
//     use pyo3::{
//         PyResult, Python,
//         types::{PyAnyMethods, PyDict, PyModule, PyModuleMethods},
//     };
//
//     #[test]
//     fn pybody_constructor_roundtrip() {
//         Python::initialize();
//         Python::attach(|py| -> PyResult<()> {
//             let m = PyModule::new(py, "test_mod")?;
//             m.add_class::<PyBody>()?;
//
//             let sys = py.import("sys").unwrap();
//             let binding = sys.getattr("modules").unwrap();
//             let modules: &Bound<PyDict> = binding.downcast().unwrap();
//             modules.set_item("test_mod", m).unwrap();
//
//             py.run(
//                 &CString::new(
//                     r#"
// from test_mod import PyBody
// # Construct with initial text
// b = PyBody("seed")
// assert b.text == "seed"
// assert b.len() == 4
// assert not b.is_empty()
//
// # Text -> raw roundtrip (includes NUL to check binary safety)
// b.text = "abc\x00def"
// assert b.len() == 7
// raw = b.raw
// assert isinstance(raw, (bytes, bytearray))
// assert raw == b"abc\x00def"
//
// # Raw -> text
// b.raw = b"hi"
// assert b.text == "hi"
// assert b.len() == 2
//
// # __repr__ should include len and a preview (don’t over-specify exact format)
// r = repr(b)
// assert "PyBody" in r and "len=2" in r
//                     "#,
//                 )
//                 .unwrap(),
//                 None,
//                 None,
//             )?;
//
//             Ok(())
//         })
//         .unwrap();
//     }
// }
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use pyo3::{
        PyResult, Python,
        types::{PyAnyMethods, PyDict, PyModule, PyModuleMethods},
    };

    /// Helper: register `PyBody` in a temp module `test_mod` and run a Python snippet.
    fn with_module(code: &str) {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let m = PyModule::new(py, "test_mod")?;
            m.add_class::<PyBody>()?;

            // sys.modules["test_mod"] = m
            let sys = py.import("sys")?;
            let modules: &Bound<PyDict> = sys.getattr("modules")?.downcast()?;
            modules.set_item("test_mod", m)?;

            py.run(&CString::new(code).unwrap(), None, None)?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn pybody_constructor_and_basic_props() {
        with_module(
            r#"
from test_mod import PyBody
# Construct with initial text
b = PyBody("seed")
assert b.text == "seed"
assert b.len() == 4
assert not b.is_empty()

# Default constructor: empty
b2 = PyBody()
assert b2.text == ""
assert b2.len() == 0
assert b2.is_empty()
"#,
        );
    }

    #[test]
    fn pybody_text_to_raw_roundtrip() {
        with_module(
            r#"
from test_mod import PyBody
b = PyBody()
# Text -> raw, include NUL to ensure binary safety
b.text = "abc\x00def"
assert b.len() == 7
raw = b.raw
assert isinstance(raw, (bytes, bytearray))
assert raw == b"abc\x00def"
"#,
        );
    }

    #[test]
    fn pybody_raw_to_text_roundtrip() {
        with_module(
            r#"
from test_mod import PyBody
b = PyBody("x")
b.raw = b"hi"
assert b.text == "hi"
assert b.len() == 2
"#,
        );
    }

    #[test]
    fn pybody_repr_contains_len_and_preview() {
        with_module(
            r#"
from test_mod import PyBody
b = PyBody("hi")
r = repr(b)
assert "PyBody" in r and "len=2" in r
"#,
        );
    }
}
