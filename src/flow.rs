use std::collections::HashMap;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Flow {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InterceptedRequest {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub url: String,
    pub version: u8,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl InterceptedRequest {
    pub fn new(
        id: i64,
        timestamp: DateTime<Utc>,
        method: String,
        url: String,
        version: u8,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> Self {
        Self {
            id,
            timestamp,
            method,
            url,
            version,
            headers,
            body,
        }
    }

    pub fn request_line(&self) -> String {
        format!("{} {} HTTP/{}", self.method, self.url, self.version)
    }
}

#[derive(Debug, Clone)]
pub struct InterceptedResponse {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub status: u16,
    pub reason: String,
    pub version: u8,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl InterceptedResponse {
    pub fn request_line(&self) -> String {
        format!("{} {} HTTP/{}", self.status, self.reason, self.version)
    }
}
