use std::io::Cursor;

use backend_core::{McpServer, MAX_MCP_MESSAGE_BYTES};
use serde_json::Value;

const PING: &str = r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#;

fn serve(input: Vec<u8>) -> String {
    let mut server = McpServer::new(None);
    let mut output = Vec::new();
    server
        .serve_io(Cursor::new(input), &mut output)
        .expect("serve_io should succeed");
    String::from_utf8(output).expect("server output should be valid UTF-8")
}

fn framed(body: &str) -> Vec<u8> {
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

fn framed_bodies(out: &str) -> Vec<Value> {
    let mut bodies = Vec::new();
    let mut rest = out;
    while !rest.is_empty() {
        let after_prefix = rest
            .strip_prefix("Content-Length: ")
            .expect("framed output must start with Content-Length header");
        let (len_str, after_header) = after_prefix
            .split_once("\r\n\r\n")
            .expect("framed output must contain header separator");
        let len: usize = len_str.trim().parse().expect("valid Content-Length usize");
        let body = &after_header[..len];
        bodies.push(serde_json::from_str(body).expect("valid JSON-RPC response body"));
        rest = &after_header[len..];
    }
    bodies
}

#[test]
fn oversized_content_length_is_rejected_without_allocating() {
    let out = serve(b"Content-Length: 1073741824\r\n\r\n".to_vec());
    let replies = framed_bodies(&out);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["id"], Value::Null);
    assert_eq!(replies[0]["error"]["code"], -32600);
    let message = replies[0]["error"]["message"]
        .as_str()
        .expect("error message string");
    assert!(
        message.contains("exceeds"),
        "expected error message to mention 'exceeds', got: {message}"
    );
}

#[test]
fn invalid_content_length_is_rejected_and_serving_continues() {
    let mut input = b"Content-Length: abc\r\n\r\n".to_vec();
    input.extend_from_slice(&framed(PING));

    let out = serve(input);
    let replies = framed_bodies(&out);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["id"], Value::Null);
    assert_eq!(replies[0]["error"]["code"], -32600);
    assert_eq!(replies[1]["id"], 7);
    assert!(replies[1].get("error").is_none());
}

#[test]
fn headers_are_accepted_in_any_order() {
    let input = format!(
        "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: {}\r\n\r\n{PING}",
        PING.len()
    )
    .into_bytes();

    let out = serve(input);
    let replies = framed_bodies(&out);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["id"], 7);
    assert!(replies[0].get("error").is_none());
}

#[test]
fn overlong_line_is_rejected_and_the_next_line_is_served() {
    let mut input = vec![b'x'; MAX_MCP_MESSAGE_BYTES + 1];
    input.push(b'\n');
    input.extend_from_slice(PING.as_bytes());
    input.push(b'\n');

    let out = serve(input);
    let lines: Vec<Value> = out
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid JSON line reply"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["id"], Value::Null);
    assert_eq!(lines[0]["error"]["code"], -32600);
    assert_eq!(lines[1]["id"], 7);
    assert!(lines[1].get("error").is_none());
}

#[test]
fn non_content_length_header_lines_are_ignored() {
    let input = format!("Content-Length: {}\r\nbogus\r\n\r\n{PING}", PING.len()).into_bytes();

    let out = serve(input);
    let replies = framed_bodies(&out);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["id"], 7);
    assert!(replies[0].get("error").is_none());
}
