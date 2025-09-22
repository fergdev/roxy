use std::fmt::Display;

use cow_utils::CowUtils;
use http::Method;
use pyo3::{basic::CompareOp, prelude::*};

#[pyclass(name = "Method")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum PyMethod {
    CONNECT,
    DELETE,
    #[default]
    GET,
    HEAD,
    OPTIONS,
    PATCH,
    POST,
    PUT,
    TRACE,
}

impl Display for PyMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&Method> for PyMethod {
    fn from(m: &Method) -> Self {
        match *m {
            Method::CONNECT => PyMethod::CONNECT,
            Method::DELETE => PyMethod::DELETE,
            Method::GET => PyMethod::GET,
            Method::HEAD => PyMethod::HEAD,
            Method::OPTIONS => PyMethod::OPTIONS,
            Method::PATCH => PyMethod::PATCH,
            Method::POST => PyMethod::POST,
            Method::PUT => PyMethod::PUT,
            Method::TRACE => PyMethod::TRACE,
            _ => PyMethod::GET,
        }
    }
}

impl From<PyMethod> for Method {
    fn from(val: PyMethod) -> Self {
        match val {
            PyMethod::CONNECT => Method::CONNECT,
            PyMethod::DELETE => Method::DELETE,
            PyMethod::GET => Method::GET,
            PyMethod::HEAD => Method::HEAD,
            PyMethod::OPTIONS => Method::OPTIONS,
            PyMethod::PATCH => Method::PATCH,
            PyMethod::POST => Method::POST,
            PyMethod::PUT => Method::PUT,
            PyMethod::TRACE => Method::TRACE,
        }
    }
}

#[pymethods]
impl PyMethod {
    #[getter]
    fn value(&self) -> &'static str {
        match self {
            PyMethod::CONNECT => "CONNECT",
            PyMethod::DELETE => "DELETE",
            PyMethod::GET => "GET",
            PyMethod::HEAD => "HEAD",
            PyMethod::OPTIONS => "OPTIONS",
            PyMethod::PATCH => "PATCH",
            PyMethod::POST => "POST",
            PyMethod::PUT => "PUT",
            PyMethod::TRACE => "TRACE",
        }
    }

    fn __str__(&self) -> &'static str {
        self.value()
    }

    fn __repr__(&self) -> String {
        format!("Method.{}", self.value())
    }

    fn __richcmp__(&self, other: Bound<PyAny>, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => {
                if let Ok(other) = other.extract::<PyRef<PyMethod>>() {
                    return Ok(self == &*other);
                }
                if let Ok(s) = other.extract::<String>() {
                    return Ok(self.value() == s);
                }
                Ok(false)
            }
            CompareOp::Ne => {
                if let Ok(other) = other.extract::<PyRef<PyMethod>>() {
                    return Ok(self != &*other);
                }
                if let Ok(s) = other.extract::<String>() {
                    return Ok(self.value() != s);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[pyclass(name = "Protocol")]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum PyProtocol {
    HTTP,
    HTTPS,
}

#[pymethods]
impl PyProtocol {
    #[getter]
    fn value(&self) -> &'static str {
        match self {
            PyProtocol::HTTP => "http",
            PyProtocol::HTTPS => "https",
        }
    }
    fn __str__(&self) -> &'static str {
        self.value()
    }
    fn __repr__(&self) -> String {
        format!("Protocol.{}", self.value().cow_to_uppercase())
    }
}

#[pyclass(name = "Version")]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum PyVersion {
    HTTP09,
    HTTP10,
    HTTP11,
    HTTP2,
    HTTP3,
}

#[pymethods]
impl PyVersion {
    #[getter]
    fn value(&self) -> &'static str {
        match self {
            PyVersion::HTTP09 => "HTTP/0.9",
            PyVersion::HTTP10 => "HTTP/1.0",
            PyVersion::HTTP11 => "HTTP/1.1",
            PyVersion::HTTP2 => "HTTP/2",
            PyVersion::HTTP3 => "HTTP/3",
        }
    }
    fn __str__(&self) -> &'static str {
        self.value()
    }
    fn __repr__(&self) -> String {
        format!("Version.{}", self.value())
    }
}
