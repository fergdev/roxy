use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use chrono::Date;
use chrono::Utc;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, trace};

use crate::event::{AppEvent, Event, RequestData, ResponseData};

pub fn start_proxy(listen_addr_in: &str, tx: UnboundedSender<Event>) -> std::io::Result<()> {
    let listen_addr = listen_addr_in.to_string();
    tokio::spawn(async move {
        let listener = TcpListener::bind(listen_addr.clone()).unwrap();
        debug!("Proxy listening on {}", listen_addr);

        let mut id_generator_generator = SnowflakeIdGenerator::new(1, 1);
        for stream in listener.incoming() {
            debug!("Proxy received req");
            let id = id_generator_generator.real_time_generate();
            let mut client = stream.unwrap();
            let mut buf_reader = std::io::BufReader::new(client.try_clone().unwrap());

            let mut request_line = String::new();
            buf_reader.read_line(&mut request_line).unwrap();

            debug!("Request line:\n{}", request_line);

            let mut headers: Vec<String> = Vec::new();

            loop {
                let mut line = String::new();
                buf_reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line == "\n" {
                    break;
                }
                headers.push(line);
            }
            debug!("Headers len:\n{}", headers.len());

            // let content_length = headers
            //     .lines()
            //     .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
            //     .and_then(|l| l.split(':').nth(1))
            //     .and_then(|v| v.trim().parse::<usize>().ok())
            //     .unwrap_or(0);
            let content_length = 0;

            // Read body
            let mut body = vec![0; content_length];
            if content_length > 0 {
                buf_reader.read_exact(&mut body).unwrap();
            }

            let body = if let Ok(text) = std::str::from_utf8(&body[..content_length]) {
                debug!("Server read {} bytes: {}", content_length, text);
                Some(text.to_string())
            } else {
                debug!("Server read {} bytes (non-UTF8 data)", content_length);
                None
            };

            if let Err(err) = tx.send(Event::App(AppEvent::Request(RequestData {
                id,
                timestamp: Utc::now(),
                request_line: request_line.clone(),
                headers: headers.clone(),
                body: body.clone(),
            }))) {
                error!("Error sending line {}", err);
            };

            let target_addr = if let Some(host_start) = request_line.find("http://") {
                let url = &request_line[host_start + 7..];
                debug!("Url line {}", url);
                if let Some(end) = url.find('/') {
                    &url[..end]
                } else {
                    url
                }
            } else {
                error!("No url in request line {}", request_line);
                "127.0.0.1:80"
            }
            .to_string();

            let target_addr_port = format!("{}:80", &target_addr);
            debug!("Target address: {}", target_addr_port);

            let tx2 = tx.clone();
            thread::spawn(move || {
                debug!("Connecting to target: {}", target_addr_port);
                if let Ok(mut server) = TcpStream::connect(target_addr_port) {
                    debug!("Connected to target");
                    let mut client_clone = client.try_clone().unwrap();
                    let mut server_clone = server.try_clone().unwrap();

                    // write request to server
                    // request_line
                    // headers
                    // body

                    let mut server_req = String::new();
                    server_req.push_str(request_line.as_str());
                    // server_req.push_str("\n");
                    for h in headers {
                        server_req.push_str(h.as_str());
                        // server_req.push_str("\n");
                    }
                    server_req.push_str("\r\n");
                    debug!("server_req");
                    debug!(server_req);
                    debug!("server_req end");

                    server_clone.write_all(server_req.as_bytes()).unwrap();

                    // debug!("Writing request_line");
                    // server_clone.write_all(request_line.as_bytes()).unwrap();
                    // server_clone.write_all("\r".as_bytes()).unwrap();
                    // debug!("Writing headers");
                    // server_clone
                    //     .write_all(headers.join("\r").as_bytes())
                    //     .unwrap();
                    // debug!("Writing body");
                    // // server_clone.write_all("\r".as_bytes()).unwrap();
                    // server_clone.write_all("\r".as_bytes()).unwrap();

                    // Server -> Client
                    let mut buf = [0u8; 4096];
                    loop {
                        let n = match server.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        let body = if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                            debug!("Server read {} bytes: {}", n, text);
                            Some(text.to_string())
                        } else {
                            debug!("Server read {} bytes (non-UTF8 data)", n);
                            None
                        };

                        tx2.send(Event::App(AppEvent::Response(ResponseData {
                            id,
                            timestamp: Utc::now(),
                            request_line: "todo".to_string(),
                            headers: "todo".to_string(),
                            body,
                        })))
                        .expect("Failed to send response data");

                        if client.write_all(&buf[..n]).is_err() {
                            break;
                        }
                    }
                    // let _ = c2s.join();
                }
            });
        }
    });
    Ok(())
}
