use std::fmt::Display;

use cow_utils::CowUtils;
use http::{Method, Version};
use pyo3::{basic::CompareOp, prelude::*};
use roxy_shared::version::HttpVersion;

#[allow(clippy::upper_case_acronyms)]
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

#[allow(clippy::upper_case_acronyms)]
#[pyclass(name = "Protocol")]
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
                    return Ok(self.value() == s);
                }
                Ok(false)
            }
            CompareOp::Ne => {
                if let Ok(other) = other.extract::<PyRef<PyProtocol>>() {
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

impl From<&str> for PyProtocol {
    fn from(value: &str) -> Self {
        match value {
            "http" => PyProtocol::HTTP,
            "https" => PyProtocol::HTTPS,
            _ => PyProtocol::HTTP,
        }
    }
}

#[pyclass(name = "Version")]
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
        to_u16(self)
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

#[allow(clippy::upper_case_acronyms)]
#[pyclass(name = "Version")]
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

impl From<u16> for PyStatus {
    fn from(value: u16) -> Self {
        match value {
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
            _ => PyStatus::OK,
        }
    }
}

fn to_u16(py_status: &PyStatus) -> u16 {
    match py_status {
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
// (100, CONTINUE, "Continue");
// (101, SWITCHING_PROTOCOLS, "Switching Protocols");
// (102, PROCESSING, "Processing");
// (200, OK, "OK");
// (201, CREATED, "Created");
// (202, ACCEPTED, "Accepted");
// (203, NON_AUTHORITATIVE_INFORMATION, "Non Authoritative Information");
// (204, NO_CONTENT, "No Content");
// (205, RESET_CONTENT, "Reset Content");
// (206, PARTIAL_CONTENT, "Partial Content");
// (207, MULTI_STATUS, "Multi-Status");
// (208, ALREADY_REPORTED, "Already Reported");
// (226, IM_USED, "IM Used");
// (300, MULTIPLE_CHOICES, "Multiple Choices");
// (301, MOVED_PERMANENTLY, "Moved Permanently");
// (302, FOUND, "Found");
// (303, SEE_OTHER, "See Other");
// (304, NOT_MODIFIED, "Not Modified");
// (305, USE_PROXY, "Use Proxy");
// (307, TEMPORARY_REDIRECT, "Temporary Redirect");
// (308, PERMANENT_REDIRECT, "Permanent Redirect");
// (400, BAD_REQUEST, "Bad Request");
// (401, UNAUTHORIZED, "Unauthorized");
// (402, PAYMENT_REQUIRED, "Payment Required");
// (403, FORBIDDEN, "Forbidden");
// (404, NOT_FOUND, "Not Found");
// (405, METHOD_NOT_ALLOWED, "Method Not Allowed");
// (406, NOT_ACCEPTABLE, "Not Acceptable");
// (407, PROXY_AUTHENTICATION_REQUIRED, "Proxy Authentication Required");
// (408, REQUEST_TIMEOUT, "Request Timeout");
// (409, CONFLICT, "Conflict");
// (410, GONE, "Gone");
// (411, LENGTH_REQUIRED, "Length Required");
// (412, PRECONDITION_FAILED, "Precondition Failed");
// (413, PAYLOAD_TOO_LARGE, "Payload Too Large");
// (414, URI_TOO_LONG, "URI Too Long");
// (415, UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type");
// (416, RANGE_NOT_SATISFIABLE, "Range Not Satisfiable");
// (417, EXPECTATION_FAILED, "Expectation Failed");
// (418, IM_A_TEAPOT, "I'm a teapot");
// (421, MISDIRECTED_REQUEST, "Misdirected Request");
// (422, UNPROCESSABLE_ENTITY, "Unprocessable Entity");
// (423, LOCKED, "Locked");
// (424, FAILED_DEPENDENCY, "Failed Dependency");
// (425, TOO_EARLY, "Too Early");
// (426, UPGRADE_REQUIRED, "Upgrade Required");
// (428, PRECONDITION_REQUIRED, "Precondition Required");
// (429, TOO_MANY_REQUESTS, "Too Many Requests");
// (431, REQUEST_HEADER_FIELDS_TOO_LARGE, "Request Header Fields Too Large");
// (451, UNAVAILABLE_FOR_LEGAL_REASONS, "Unavailable For Legal Reasons");
// (500, INTERNAL_SERVER_ERROR, "Internal Server Error");
// (501, NOT_IMPLEMENTED, "Not Implemented");
// (502, BAD_GATEWAY, "Bad Gateway");
// (503, SERVICE_UNAVAILABLE, "Service Unavailable");
// (504, GATEWAY_TIMEOUT, "Gateway Timeout");
// (505, HTTP_VERSION_NOT_SUPPORTED, "HTTP Version Not Supported");
// (506, VARIANT_ALSO_NEGOTIATES, "Variant Also Negotiates");
// (507, INSUFFICIENT_STORAGE, "Insufficient Storage");
// (508, LOOP_DETECTED, "Loop Detected");
// (510, NOT_EXTENDED, "Not Extended");
// (511, NETWORK_AUTHENTICATION_REQUIRED, "Network Authentication Required");
