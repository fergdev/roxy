use roxy_shared::cert::CapturedClientHello;

pub(crate) fn process_client_hello(
    client_hello: &Option<CapturedClientHello>,
) -> Vec<(String, String)> {
    match client_hello {
        Some(capture) => {
            vec![
                (
                    "Server name".to_string(),
                    capture
                        .server_name
                        .clone()
                        .unwrap_or("None".to_string())
                        .to_string(),
                ),
                (
                    "Signature schemes".to_string(),
                    if capture.signature_schemes.is_empty() {
                        "None".to_string()
                    } else {
                        capture
                            .signature_schemes
                            .iter()
                            .map(|s| format!("{s:?}").to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    },
                ),
                (
                    "ALPN".to_string(),
                    if let Some(alpn) = &capture.alpn {
                        if alpn.is_empty() {
                            "Empty".to_string()
                        } else {
                            alpn.iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        }
                    } else {
                        "None".to_string()
                    },
                ),
                (
                    "server_cert_types".to_string(),
                    if let Some(server_cert_types) = &capture.server_cert_types {
                        if server_cert_types.is_empty() {
                            "Empty".to_string()
                        } else {
                            server_cert_types
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        }
                    } else {
                        "None".to_string()
                    },
                ),
                (
                    "client_cert_types".to_string(),
                    if let Some(client_cert_types) = &capture.server_cert_types {
                        if client_cert_types.is_empty() {
                            "Empty".to_string()
                        } else {
                            client_cert_types
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        }
                    } else {
                        "None".to_string()
                    },
                ),
                (
                    "cipher_suites".to_string(),
                    if capture.cipher_suites.is_empty() {
                        "Empty".to_string()
                    } else {
                        capture
                            .cipher_suites
                            .iter()
                            .map(|s| format!("{s:?}").to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    },
                ),
                (
                    "certificate_authorities".to_string(),
                    if let Some(certificate_authorities) = &capture.certificate_authorities {
                        if certificate_authorities.is_empty() {
                            "Empty".to_string()
                        } else {
                            certificate_authorities
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        }
                    } else {
                        "None".to_string()
                    },
                ),
                (
                    "named_groups".to_string(),
                    if let Some(named_groups) = &capture.named_groups {
                        if named_groups.is_empty() {
                            "Empty".to_string()
                        } else {
                            named_groups
                                .iter()
                                .map(|s| format!("{s:?}").to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        }
                    } else {
                        "None".to_string()
                    },
                ),
            ]
        }
        None => vec![("No data".to_string(), "".to_string())],
    }
}
