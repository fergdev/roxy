use roxy_shared::cert::{CapturedClientHello, ServerTlsConnectionData};

pub(crate) fn process_client_hello(
    client_hello: &Option<CapturedClientHello>,
) -> Vec<(String, String)> {
    let mut lines = vec![];
    match client_hello {
        Some(capture) => {
            lines.push((
                "Server name".to_string(),
                if let Some(server_name) = &capture.server_name {
                    server_name.to_string()
                } else {
                    "None".to_string()
                },
            ));

            lines.push((
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
            ));
            lines.push((
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
            ));

            lines.push((
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
            ));

            lines.push((
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
            ));
            lines.push((
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
            ));
            lines.push((
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
            ));

            lines.push((
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
            ));
        }
        None => lines.push(("No data".to_string(), "".to_string())),
    }
    lines
}

pub(crate) fn process_client_tls(data: &Option<ServerTlsConnectionData>) -> Vec<(String, String)> {
    let mut lines: Vec<(String, String)> = vec![];

    match data {
        Some(capture) => {
            lines.push((
                "protocol_version".to_string(),
                match capture.protocol_version {
                    Some(version) => format!("{version:?}"),
                    None => "None".to_string(),
                },
            ));

            lines.push((
                "cipher_suite".to_string(),
                match capture.cipher_suite {
                    Some(cipher_suite) => format!("{cipher_suite:?}"),
                    None => "None".to_string(),
                },
            ));

            lines.push(("sni".to_string(), format!("{:?}", capture.sni)));

            lines.push((
                "key_exchange_group".to_string(),
                match &capture.key_exchange_group {
                    Some(key_exchange_group) => format!("{key_exchange_group:?}"),
                    None => "None".to_string(),
                },
            ));
            lines.push(("alpn".to_string(), format!("{:?}", capture.alpn)));
        }
        None => {
            lines.push(("No data".to_string(), String::new()));
        }
    }
    lines
}
