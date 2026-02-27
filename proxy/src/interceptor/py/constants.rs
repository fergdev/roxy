use std::fmt::Display;

use cow_utils::CowUtils;
use http::{Method, StatusCode, Version};
use pyo3::{basic::CompareOp, prelude::*};
use roxy_shared::version::HttpVersion;

#[allow(clippy::upper_case_acronyms)]
#[pyclass(from_py_object, name = "Method")]
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
                    return Ok(self.value().cow_to_lowercase() != s.cow_to_lowercase());
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[pyclass(from_py_object, name = "Protocol")]
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum PyProtocol {
    #[default]
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

    fn __richcmp__(&self, other: Bound<PyAny>, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => {
                if let Ok(other) = other.extract::<PyRef<PyProtocol>>() {
                    return Ok(self == &*other);
                }
                if let Ok(s) = other.extract::<String>() {
                    return Ok(self.value().cow_to_lowercase() == s.cow_to_lowercase());
                }
                Ok(false)
            }
            CompareOp::Ne => {
                if let Ok(other) = other.extract::<PyRef<PyProtocol>>() {
                    return Ok(self != &*other);
                }
                if let Ok(s) = other.extract::<String>() {
                    return Ok(self.value().cow_to_lowercase() != s.cow_to_lowercase());
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl Display for PyProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for PyProtocol {
    fn from(value: &str) -> Self {
        match value.cow_to_lowercase().clone().as_ref() {
            "http" => PyProtocol::HTTP,
            "https" => PyProtocol::HTTPS,
            _ => PyProtocol::HTTP,
        }
    }
}

#[pyclass(from_py_object, name = "Version")]
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum PyVersion {
    HTTP09,
    HTTP10,
    #[default]
    HTTP11,
    HTTP2,
    HTTP3,
}

impl From<&HttpVersion> for PyVersion {
    fn from(value: &HttpVersion) -> Self {
        match value.0 {
            http::Version::HTTP_09 => PyVersion::HTTP09,
            http::Version::HTTP_10 => PyVersion::HTTP10,
            http::Version::HTTP_11 => PyVersion::HTTP11,
            http::Version::HTTP_2 => PyVersion::HTTP2,
            http::Version::HTTP_3 => PyVersion::HTTP3,
            _ => PyVersion::HTTP11,
        }
    }
}

impl From<PyVersion> for HttpVersion {
    fn from(py: PyVersion) -> Self {
        match py {
            PyVersion::HTTP09 => HttpVersion(Version::HTTP_09),
            PyVersion::HTTP10 => HttpVersion(Version::HTTP_10),
            PyVersion::HTTP11 => HttpVersion(Version::HTTP_11),
            PyVersion::HTTP2 => HttpVersion(Version::HTTP_2),
            PyVersion::HTTP3 => HttpVersion(Version::HTTP_3),
        }
    }
}

#[pymethods]
impl PyVersion {
    #[getter]
    fn value(&self) -> &'static str {
        match self {
            PyVersion::HTTP09 => "HTTP/0.9",
            PyVersion::HTTP10 => "HTTP/1.0",
            PyVersion::HTTP11 => "HTTP/1.1",
            PyVersion::HTTP2 => "HTTP/2.0",
            PyVersion::HTTP3 => "HTTP/3.0",
        }
    }
    fn __str__(&self) -> &'static str {
        self.value()
    }
    fn __repr__(&self) -> String {
        format!("Version.{}", self.value())
    }
    fn __richcmp__(&self, other: Bound<PyAny>, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => {
                if let Ok(other) = other.extract::<PyRef<PyVersion>>() {
                    return Ok(self == &*other);
                }
                if let Ok(s) = other.extract::<String>() {
                    return Ok(self.value() == s);
                }
                Ok(false)
            }
            CompareOp::Ne => {
                if let Ok(other) = other.extract::<PyRef<PyVersion>>() {
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

#[pymethods]
impl PyStatus {
    #[getter]
    fn value(&self) -> u16 {
        self.clone().into()
    }
    fn __str__(&self) -> String {
        format!("{self:?}")
    }
    fn __repr__(&self) -> String {
        format!("Status.{self:?}")
    }
    fn __richcmp__(&self, other: Bound<PyAny>, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => {
                if let Ok(other) = other.extract::<PyRef<PyStatus>>() {
                    return Ok(self == &*other);
                }
                if let Ok(s) = other.extract::<u16>() {
                    return Ok(self.value() == s);
                }
                Ok(false)
            }
            CompareOp::Ne => {
                if let Ok(other) = other.extract::<PyRef<PyStatus>>() {
                    return Ok(self != &*other);
                }
                if let Ok(s) = other.extract::<u16>() {
                    return Ok(self.value() != s);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[pyclass(from_py_object, name = "Status")]
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub(crate) enum PyStatus {
    CONTINUE,
    SWITCHING_PROTOCOLS,
    PROCESSING,
    #[default]
    OK,
    CREATED,
    ACCEPTED,
    NON_AUTHORITATIVE_INFORMATION,
    NO_CONTENT,
    RESET_CONTENT,
    PARTIAL_CONTENT,
    MULTI_STATUS,
    ALREADY_REPORTED,
    IM_USED,
    MULTIPLE_CHOICES,
    MOVED_PERMANENTLY,
    FOUND,
    SEE_OTHER,
    NOT_MODIFIED,
    USE_PROXY,
    TEMPORARY_REDIRECT,
    PERMANENT_REDIRECT,
    BAD_REQUEST,
    UNAUTHORIZED,
    PAYMENT_REQUIRED,
    FORBIDDEN,
    NOT_FOUND,
    METHOD_NOT_ALLOWED,
    NOT_ACCEPTABLE,
    PROXY_AUTHENTICATION_REQUIRED,
    REQUEST_TIMEOUT,
    CONFLICT,
    GONE,
    LENGTH_REQUIRED,
    PRECONDITION_FAILED,
    PAYLOAD_TOO_LARGE,
    URI_TOO_LONG,
    UNSUPPORTED_MEDIA_TYPE,
    RANGE_NOT_SATISFIABLE,
    EXPECTATION_FAILED,
    IM_A_TEAPOT,
    MISDIRECTED_REQUEST,
    UNPROCESSABLE_ENTITY,
    LOCKED,
    FAILED_DEPENDENCY,
    TOO_EARLY,
    UPGRADE_REQUIRED,
    PRECONDITION_REQUIRED,
    TOO_MANY_REQUESTS,
    REQUEST_HEADER_FIELDS_TOO_LARGE,
    UNAVAILABLE_FOR_LEGAL_REASONS,
    INTERNAL_SERVER_ERROR,
    NOT_IMPLEMENTED,
    BAD_GATEWAY,
    SERVICE_UNAVAILABLE,
    GATEWAY_TIMEOUT,
    HTTP_VERSION_NOT_SUPPORTED,
    VARIANT_ALSO_NEGOTIATES,
    INSUFFICIENT_STORAGE,
    LOOP_DETECTED,
    NOT_EXTENDED,
    NETWORK_AUTHENTICATION_REQUIRED,
}

impl From<PyStatus> for u16 {
    fn from(val: PyStatus) -> Self {
        match val {
            PyStatus::CONTINUE => 100,
            PyStatus::SWITCHING_PROTOCOLS => 101,
            PyStatus::PROCESSING => 102,
            PyStatus::OK => 200,
            PyStatus::CREATED => 201,
            PyStatus::ACCEPTED => 202,
            PyStatus::NON_AUTHORITATIVE_INFORMATION => 203,
            PyStatus::NO_CONTENT => 204,
            PyStatus::RESET_CONTENT => 205,
            PyStatus::PARTIAL_CONTENT => 206,
            PyStatus::MULTI_STATUS => 207,
            PyStatus::ALREADY_REPORTED => 208,
            PyStatus::IM_USED => 226,
            PyStatus::MULTIPLE_CHOICES => 300,
            PyStatus::MOVED_PERMANENTLY => 301,
            PyStatus::FOUND => 302,
            PyStatus::SEE_OTHER => 303,
            PyStatus::NOT_MODIFIED => 304,
            PyStatus::USE_PROXY => 305,
            PyStatus::TEMPORARY_REDIRECT => 307,
            PyStatus::PERMANENT_REDIRECT => 308,
            PyStatus::BAD_REQUEST => 400,
            PyStatus::UNAUTHORIZED => 401,
            PyStatus::PAYMENT_REQUIRED => 402,
            PyStatus::FORBIDDEN => 403,
            PyStatus::NOT_FOUND => 404,
            PyStatus::METHOD_NOT_ALLOWED => 405,
            PyStatus::NOT_ACCEPTABLE => 406,
            PyStatus::PROXY_AUTHENTICATION_REQUIRED => 407,
            PyStatus::REQUEST_TIMEOUT => 408,
            PyStatus::CONFLICT => 409,
            PyStatus::GONE => 410,
            PyStatus::LENGTH_REQUIRED => 411,
            PyStatus::PRECONDITION_FAILED => 412,
            PyStatus::PAYLOAD_TOO_LARGE => 413,
            PyStatus::URI_TOO_LONG => 414,
            PyStatus::UNSUPPORTED_MEDIA_TYPE => 415,
            PyStatus::RANGE_NOT_SATISFIABLE => 416,
            PyStatus::EXPECTATION_FAILED => 417,
            PyStatus::IM_A_TEAPOT => 418,
            PyStatus::MISDIRECTED_REQUEST => 421,
            PyStatus::UNPROCESSABLE_ENTITY => 422,
            PyStatus::LOCKED => 423,
            PyStatus::FAILED_DEPENDENCY => 424,
            PyStatus::TOO_EARLY => 425,
            PyStatus::UPGRADE_REQUIRED => 426,
            PyStatus::PRECONDITION_REQUIRED => 428,
            PyStatus::TOO_MANY_REQUESTS => 429,
            PyStatus::REQUEST_HEADER_FIELDS_TOO_LARGE => 431,
            PyStatus::UNAVAILABLE_FOR_LEGAL_REASONS => 451,
            PyStatus::INTERNAL_SERVER_ERROR => 500,
            PyStatus::NOT_IMPLEMENTED => 501,
            PyStatus::BAD_GATEWAY => 502,
            PyStatus::SERVICE_UNAVAILABLE => 503,
            PyStatus::GATEWAY_TIMEOUT => 504,
            PyStatus::HTTP_VERSION_NOT_SUPPORTED => 505,
            PyStatus::VARIANT_ALSO_NEGOTIATES => 506,
            PyStatus::INSUFFICIENT_STORAGE => 507,
            PyStatus::LOOP_DETECTED => 508,
            PyStatus::NOT_EXTENDED => 510,
            PyStatus::NETWORK_AUTHENTICATION_REQUIRED => 511,
        }
    }
}

impl TryFrom<u16> for PyStatus {
    type Error = PyErr;

    fn try_from(value: u16) -> Result<Self, PyErr> {
        let v = match value {
            100 => PyStatus::CONTINUE,
            101 => PyStatus::SWITCHING_PROTOCOLS,
            102 => PyStatus::PROCESSING,
            200 => PyStatus::OK,
            201 => PyStatus::CREATED,
            202 => PyStatus::ACCEPTED,
            203 => PyStatus::NON_AUTHORITATIVE_INFORMATION,
            204 => PyStatus::NO_CONTENT,
            205 => PyStatus::RESET_CONTENT,
            206 => PyStatus::PARTIAL_CONTENT,
            207 => PyStatus::MULTI_STATUS,
            208 => PyStatus::ALREADY_REPORTED,
            226 => PyStatus::IM_USED,
            300 => PyStatus::MULTIPLE_CHOICES,
            301 => PyStatus::MOVED_PERMANENTLY,
            302 => PyStatus::FOUND,
            303 => PyStatus::SEE_OTHER,
            304 => PyStatus::NOT_MODIFIED,
            305 => PyStatus::USE_PROXY,
            307 => PyStatus::TEMPORARY_REDIRECT,
            308 => PyStatus::PERMANENT_REDIRECT,
            400 => PyStatus::BAD_REQUEST,
            401 => PyStatus::UNAUTHORIZED,
            402 => PyStatus::PAYMENT_REQUIRED,
            403 => PyStatus::FORBIDDEN,
            404 => PyStatus::NOT_FOUND,
            405 => PyStatus::METHOD_NOT_ALLOWED,
            406 => PyStatus::NOT_ACCEPTABLE,
            407 => PyStatus::PROXY_AUTHENTICATION_REQUIRED,
            408 => PyStatus::REQUEST_TIMEOUT,
            409 => PyStatus::CONFLICT,
            410 => PyStatus::GONE,
            411 => PyStatus::LENGTH_REQUIRED,
            412 => PyStatus::PRECONDITION_FAILED,
            413 => PyStatus::PAYLOAD_TOO_LARGE,
            414 => PyStatus::URI_TOO_LONG,
            415 => PyStatus::UNSUPPORTED_MEDIA_TYPE,
            416 => PyStatus::RANGE_NOT_SATISFIABLE,
            417 => PyStatus::EXPECTATION_FAILED,
            418 => PyStatus::IM_A_TEAPOT,
            421 => PyStatus::MISDIRECTED_REQUEST,
            422 => PyStatus::UNPROCESSABLE_ENTITY,
            423 => PyStatus::LOCKED,
            424 => PyStatus::FAILED_DEPENDENCY,
            425 => PyStatus::TOO_EARLY,
            426 => PyStatus::UPGRADE_REQUIRED,
            428 => PyStatus::PRECONDITION_REQUIRED,
            429 => PyStatus::TOO_MANY_REQUESTS,
            431 => PyStatus::REQUEST_HEADER_FIELDS_TOO_LARGE,
            451 => PyStatus::UNAVAILABLE_FOR_LEGAL_REASONS,
            500 => PyStatus::INTERNAL_SERVER_ERROR,
            501 => PyStatus::NOT_IMPLEMENTED,
            502 => PyStatus::BAD_GATEWAY,
            503 => PyStatus::SERVICE_UNAVAILABLE,
            504 => PyStatus::GATEWAY_TIMEOUT,
            505 => PyStatus::HTTP_VERSION_NOT_SUPPORTED,
            506 => PyStatus::VARIANT_ALSO_NEGOTIATES,
            507 => PyStatus::INSUFFICIENT_STORAGE,
            508 => PyStatus::LOOP_DETECTED,
            510 => PyStatus::NOT_EXTENDED,
            511 => PyStatus::NETWORK_AUTHENTICATION_REQUIRED,
            _ => {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "method must be Method enum or string",
                ));
            }
        };
        Ok(v)
    }
}

impl From<StatusCode> for PyStatus {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::CONTINUE => PyStatus::CONTINUE,
            StatusCode::SWITCHING_PROTOCOLS => PyStatus::SWITCHING_PROTOCOLS,
            StatusCode::PROCESSING => PyStatus::PROCESSING,
            StatusCode::OK => PyStatus::OK,
            StatusCode::CREATED => PyStatus::CREATED,
            StatusCode::ACCEPTED => PyStatus::ACCEPTED,
            StatusCode::NON_AUTHORITATIVE_INFORMATION => PyStatus::NON_AUTHORITATIVE_INFORMATION,
            StatusCode::NO_CONTENT => PyStatus::NO_CONTENT,
            StatusCode::RESET_CONTENT => PyStatus::RESET_CONTENT,
            StatusCode::PARTIAL_CONTENT => PyStatus::PARTIAL_CONTENT,
            StatusCode::MULTI_STATUS => PyStatus::MULTI_STATUS,
            StatusCode::ALREADY_REPORTED => PyStatus::ALREADY_REPORTED,
            StatusCode::IM_USED => PyStatus::IM_USED,
            StatusCode::MULTIPLE_CHOICES => PyStatus::MULTIPLE_CHOICES,
            StatusCode::MOVED_PERMANENTLY => PyStatus::MOVED_PERMANENTLY,
            StatusCode::FOUND => PyStatus::FOUND,
            StatusCode::SEE_OTHER => PyStatus::SEE_OTHER,
            StatusCode::NOT_MODIFIED => PyStatus::NOT_MODIFIED,
            StatusCode::USE_PROXY => PyStatus::USE_PROXY,
            StatusCode::TEMPORARY_REDIRECT => PyStatus::TEMPORARY_REDIRECT,
            StatusCode::PERMANENT_REDIRECT => PyStatus::PERMANENT_REDIRECT,
            StatusCode::BAD_REQUEST => PyStatus::BAD_REQUEST,
            StatusCode::UNAUTHORIZED => PyStatus::UNAUTHORIZED,
            StatusCode::PAYMENT_REQUIRED => PyStatus::PAYMENT_REQUIRED,
            StatusCode::FORBIDDEN => PyStatus::FORBIDDEN,
            StatusCode::NOT_FOUND => PyStatus::NOT_FOUND,
            StatusCode::METHOD_NOT_ALLOWED => PyStatus::METHOD_NOT_ALLOWED,
            StatusCode::NOT_ACCEPTABLE => PyStatus::NOT_ACCEPTABLE,
            StatusCode::PROXY_AUTHENTICATION_REQUIRED => PyStatus::PROXY_AUTHENTICATION_REQUIRED,
            StatusCode::REQUEST_TIMEOUT => PyStatus::REQUEST_TIMEOUT,
            StatusCode::CONFLICT => PyStatus::CONFLICT,
            StatusCode::GONE => PyStatus::GONE,
            StatusCode::LENGTH_REQUIRED => PyStatus::LENGTH_REQUIRED,
            StatusCode::PRECONDITION_FAILED => PyStatus::PRECONDITION_FAILED,
            StatusCode::PAYLOAD_TOO_LARGE => PyStatus::PAYLOAD_TOO_LARGE,
            StatusCode::URI_TOO_LONG => PyStatus::URI_TOO_LONG,
            StatusCode::UNSUPPORTED_MEDIA_TYPE => PyStatus::UNSUPPORTED_MEDIA_TYPE,
            StatusCode::RANGE_NOT_SATISFIABLE => PyStatus::RANGE_NOT_SATISFIABLE,
            StatusCode::EXPECTATION_FAILED => PyStatus::EXPECTATION_FAILED,
            StatusCode::IM_A_TEAPOT => PyStatus::IM_A_TEAPOT,
            StatusCode::MISDIRECTED_REQUEST => PyStatus::MISDIRECTED_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY => PyStatus::UNPROCESSABLE_ENTITY,
            StatusCode::LOCKED => PyStatus::LOCKED,
            StatusCode::FAILED_DEPENDENCY => PyStatus::FAILED_DEPENDENCY,
            StatusCode::TOO_EARLY => PyStatus::TOO_EARLY,
            StatusCode::UPGRADE_REQUIRED => PyStatus::UPGRADE_REQUIRED,
            StatusCode::PRECONDITION_REQUIRED => PyStatus::PRECONDITION_REQUIRED,
            StatusCode::TOO_MANY_REQUESTS => PyStatus::TOO_MANY_REQUESTS,
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE => {
                PyStatus::REQUEST_HEADER_FIELDS_TOO_LARGE
            }
            StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS => PyStatus::UNAVAILABLE_FOR_LEGAL_REASONS,
            StatusCode::INTERNAL_SERVER_ERROR => PyStatus::INTERNAL_SERVER_ERROR,
            StatusCode::NOT_IMPLEMENTED => PyStatus::NOT_IMPLEMENTED,
            StatusCode::BAD_GATEWAY => PyStatus::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE => PyStatus::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT => PyStatus::GATEWAY_TIMEOUT,
            StatusCode::HTTP_VERSION_NOT_SUPPORTED => PyStatus::HTTP_VERSION_NOT_SUPPORTED,
            StatusCode::VARIANT_ALSO_NEGOTIATES => PyStatus::VARIANT_ALSO_NEGOTIATES,
            StatusCode::INSUFFICIENT_STORAGE => PyStatus::INSUFFICIENT_STORAGE,
            StatusCode::LOOP_DETECTED => PyStatus::LOOP_DETECTED,
            StatusCode::NOT_EXTENDED => PyStatus::NOT_EXTENDED,
            StatusCode::NETWORK_AUTHENTICATION_REQUIRED => {
                PyStatus::NETWORK_AUTHENTICATION_REQUIRED
            }
            // If new codes appear in http crate but not in your enum
            _ => PyStatus::INTERNAL_SERVER_ERROR,
        }
    }
}
