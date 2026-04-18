use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tempfile::tempdir;

mod support;

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn read_request(
    stream: &mut TcpStream,
) -> Result<(String, String, HashMap<String, String>, Vec<u8>)> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(anyhow!(
                "connection closed before request headers completed"
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = find_header_end(&buffer) {
            break position + 4;
        }
    };

    let header_text = String::from_utf8(buffer[..header_end].to_vec())?;
    let mut lines = header_text.split("\r\n").filter(|line| !line.is_empty());
    let request_line = lines
        .next()
        .ok_or_else(|| anyhow!("missing HTTP request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| anyhow!("missing request method"))?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| anyhow!("missing request path"))?
        .to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok((method, path, headers, body))
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<()> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");

    stream.write_all(response.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn spawn_http_server() -> Result<(String, thread::JoinHandle<Result<()>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let base_url = format!("http://{address}/");

    let handle = thread::spawn(move || -> Result<()> {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept()?;
            let (method, path, headers, body) = read_request(&mut stream)?;

            match (method.as_str(), path.as_str()) {
                ("GET", "/hello?lang=tea&mode=tool") => {
                    let payload = br#"{"message":"hi","query":"lang=tea&mode=tool"}"#;
                    write_response(
                        &mut stream,
                        "200 OK",
                        &[("Content-Type", "application/json")],
                        payload,
                    )?;
                }
                ("POST", "/echo") => {
                    assert_eq!(
                        headers.get("content-type").map(String::as_str),
                        Some("application/json")
                    );
                    write_response(
                        &mut stream,
                        "201 Created",
                        &[("Content-Type", "application/json"), ("X-Method", "POST")],
                        &body,
                    )?;
                }
                ("GET", "/download") => {
                    let payload = [0_u8, 1, 2, 255];
                    write_response(
                        &mut stream,
                        "200 OK",
                        &[("Content-Type", "application/octet-stream")],
                        &payload,
                    )?;
                }
                _ => {
                    write_response(&mut stream, "404 Not Found", &[], b"not found")?;
                }
            }
        }
        Ok(())
    });

    Ok((base_url, handle))
}

#[test]
fn http_and_url_helpers_execute_end_to_end() -> Result<()> {
    let tmp = tempdir()?;
    let download_path = tmp.path().join("download.bin");
    let download_str = download_path.to_string_lossy();

    let (base_url, server) = spawn_http_server()?;
    let source = format!(
        r#"
use assert from "std.assert"
use fs from "std.fs"
use http from "std.http"
use url from "std.url"

const base_url = "{base_url}"
const hello_url = url.append_query(
  url.join(base_url, "hello"),
  {{"mode": "tool", "lang": "tea"}}
)

assert.eq(url.encode_component("tea lang"), "tea%20lang")
assert.eq(url.decode_component("tea%20lang"), "tea lang")
assert.eq(url.build_query({{"mode": "tool", "lang": "tea"}}), "lang=tea&mode=tool")

const hello = http.check(http.get(hello_url))
assert.eq(hello.status, 200)
assert.eq(http.header_or(hello, "content-type", ""), "application/json")

var hello_body: Dict[String, String] = http.decode_json[Dict[String, String]](hello)
assert.eq(hello_body["message"], "hi")
assert.eq(hello_body["query"], "lang=tea&mode=tool")

const echoed = http.check(http.post_json(url.join(base_url, "echo"), {{"hello": "tea"}}))
assert.eq(echoed.status, 201)
assert.eq(http.header_or(echoed, "x-method", ""), "POST")

var echoed_body: Dict[String, String] = http.decode_json[Dict[String, String]](echoed)
assert.eq(echoed_body["hello"], "tea")

const downloaded = http.download(url.join(base_url, "download"), "{download_path}")
assert.eq(downloaded.status, 200)
assert.eq(@len(downloaded.body_bytes), 4)

const bytes = fs.read_bytes("{download_path}")
assert.eq(@len(bytes), 4)
assert.eq(bytes[0], 0)
assert.eq(bytes[1], 1)
assert.eq(bytes[2], 2)
assert.eq(bytes[3], 255)

fs.remove("{download_path}")
@println("ok")
"#,
        base_url = base_url,
        download_path = download_str,
    );

    let stdout = support::build_and_run(&source, "http.tea", &[])?;
    assert_eq!(stdout, "ok\n");

    server.join().expect("http server thread should join")?;
    Ok(())
}
