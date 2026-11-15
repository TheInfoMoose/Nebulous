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

fn reply(raw: &str) -> Option<Value> {
    McpServer::new(None)
        .handle_raw_message(raw)
        .map(|out| serde_json::from_str(&out).expect("reply should be valid JSON"))
}

#[test]
fn idless_requests_get_no_reply() {
    assert_eq!(reply(r#"{"jsonrpc":"2.0","method":"ping"}"#), None);
    assert_eq!(
        reply(r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"no_such_tool"}}"#),
        None
    );
    assert_eq!(
        reply(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#),
        None
    );
}

#[test]
fn notification_only_batch_gets_no_reply() {
    assert_eq!(
        reply(
            r#"[{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","method":"ping"}]"#
        ),
        None
    );
}

#[test]
fn mixed_batch_replies_only_to_requests_with_id() {
    let out =
        reply(r#"[{"jsonrpc":"2.0","method":"ping"},{"jsonrpc":"2.0","id":"a","method":"ping"}]"#)
            .expect("expected batch reply");
    let arr = out.as_array().expect("batch reply should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "a");
    assert!(arr[0].get("error").is_none());
}

#[test]
fn invalid_request_keeps_readable_id() {
    let r1 = reply(r#"{"jsonrpc":"2.0","id":9}"#).expect("expected error reply");
    assert_eq!(r1["error"]["code"], -32600);
    assert_eq!(r1["id"], 9);

    let r2 = reply(r#"{"jsonrpc":"2.0","id":"x","method":5}"#).expect("expected error reply");
    assert_eq!(r2["error"]["code"], -32600);
    assert_eq!(r2["id"], "x");

    let r3 = reply(r#"{"jsonrpc":"2.0","method":5}"#).expect("expected error reply");
    assert_eq!(r3["error"]["code"], -32600);
    assert_eq!(r3["id"], Value::Null);
}

#[test]
fn missing_or_wrong_jsonrpc_is_rejected() {
    let r1 = reply(r#"{"id":3,"method":"ping"}"#).expect("expected error reply");
    assert_eq!(r1["error"]["code"], -32600);
    assert_eq!(r1["id"], 3);
    assert!(r1["error"]["message"]
        .as_str()
        .expect("error message string")
        .contains("jsonrpc"));

    let r2 = reply(r#"{"jsonrpc":"1.0","id":4,"method":"ping"}"#).expect("expected error reply");
    assert_eq!(r2["error"]["code"], -32600);
    assert_eq!(r2["id"], 4);

    let r3 = reply(r#"[{"id":5,"method":"ping"}]"#).expect("expected batch error reply");
    let arr = r3.as_array().expect("batch reply should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["error"]["code"], -32600);
    assert_eq!(arr[0]["id"], 5);
}

#[test]
fn unreadable_id_is_rejected_with_null_id() {
    for raw in [
        r#"{"jsonrpc":"2.0","id":{"a":1},"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#,
    ] {
        let r = reply(raw).expect("expected error reply");
        assert_eq!(r["error"]["code"], -32600);
        assert_eq!(r["id"], Value::Null);
    }
}

#[test]
fn invalid_batch_entries_each_get_an_error() {
    let out =
        reply(r#"[1,{"jsonrpc":"2.0","id":2,"method":"ping"}]"#).expect("expected batch reply");
    let arr = out.as_array().expect("batch reply should be an array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["error"]["code"], -32600);
    assert_eq!(arr[0]["id"], Value::Null);
    assert_eq!(arr[1]["id"], 2);
    assert!(arr[1].get("error").is_none());
}
