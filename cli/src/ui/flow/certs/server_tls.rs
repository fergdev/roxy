use roxy_shared::cert::ClientTlsConnectionData;

pub(crate) fn process_server_tls(data: &Option<ClientTlsConnectionData>) -> Vec<(String, String)> {
    let mut lines = vec![];

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

            lines.push((
                "ech_status".to_string(),
                format!("{:?}", capture.ech_status),
            ));

            lines.push((
                "key_exchange_group".to_string(),
                match &capture.key_exchange_group {
                    Some(key_exchange_group) => format!("{key_exchange_group:?}"),
                    None => "None".to_string(),
                },
            ));
            lines.push(("alpn".to_string(), format!("{:?}", &capture.alpn)));
        }
        None => {
            lines.push(("No data".to_string(), "".to_string()));
        }
    }
    lines
}
