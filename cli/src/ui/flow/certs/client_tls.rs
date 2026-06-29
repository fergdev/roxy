use roxy_shared::cert::ServerTlsConnectionData;

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
