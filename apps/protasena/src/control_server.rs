//! Loopback JSON-lines control server: one request per line, one reply per line, on
//! `127.0.0.1:<port>` only.
//!
//! The server reads requests only: a line that is not a JSON object with a string `method` (an
//! HTTP request line, binary data) gets one error reply and the connection is closed. Every
//! resource a client can hold is bounded ([`Limits`]): connections, line length, queued requests
//! and how long a socket may idle or block.

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

use protasena_ui_egui::ControlRequest;
use serde_json::{Value, json};

/// Resource bounds.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Connections served at once.
    pub max_clients: usize,
    /// Longest request line in bytes.
    pub max_line: usize,
    /// Requests waiting for the window at once.
    pub queue: usize,
    /// How long a connection may send nothing.
    pub idle_timeout: Duration,
    /// How long a reply write may block.
    pub write_timeout: Duration,
    /// How long a request waits for the window.
    pub reply_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_clients: 8,
            max_line: 16 * 1024 * 1024,
            queue: 64,
            idle_timeout: Duration::from_secs(600),
            write_timeout: Duration::from_secs(30),
            reply_timeout: Duration::from_secs(60),
        }
    }
}

/// Starts the server on `port`; requests arrive on the receiver. A port that cannot be bound is
/// reported and gives a receiver that never yields.
pub fn start(port: u16, wake: impl Fn() + Send + Sync + 'static) -> Receiver<ControlRequest> {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => {
            eprintln!("protasena: control channel on 127.0.0.1:{port}");
            serve(l, Limits::default(), Arc::new(wake))
        }
        Err(e) => {
            eprintln!("protasena: the control channel could not use 127.0.0.1:{port}: {e}");
            sync_channel::<ControlRequest>(0).1
        }
    }
}

/// Accepts connections on `listener` under `limits`.
pub fn serve(listener: TcpListener, limits: Limits, wake: Arc<dyn Fn() + Send + Sync>) -> Receiver<ControlRequest> {
    let (tx, rx) = sync_channel::<ControlRequest>(limits.queue.max(1));
    let _ = std::thread::Builder::new().name("control-accept".into()).spawn(move || accept(listener, tx, wake, limits));
    rx
}

struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn accept(listener: TcpListener, tx: SyncSender<ControlRequest>, wake: Arc<dyn Fn() + Send + Sync>, limits: Limits) {
    let open = Arc::new(AtomicUsize::new(0));
    for mut stream in listener.incoming().flatten() {
        let _ = stream.set_write_timeout(Some(limits.write_timeout));
        if open.fetch_add(1, Ordering::SeqCst) >= limits.max_clients {
            open.fetch_sub(1, Ordering::SeqCst);
            let _ = writeln!(stream, "{}", json!({"ok": false, "error": "too many control connections"}));
            continue;
        }
        let slot = Slot(Arc::clone(&open));
        let (tx, wake) = (tx.clone(), Arc::clone(&wake));
        let _ = std::thread::Builder::new().name("control-conn".into()).spawn(move || {
            let _slot = slot;
            connection(stream, &tx, &*wake, limits);
        });
    }
}

/// Reads one line of at most `max` bytes (newline stripped); `Ok(false)` at end of stream.
fn read_line(reader: &mut impl BufRead, buf: &mut Vec<u8>, max: usize) -> Result<bool, String> {
    buf.clear();
    let limit = u64::try_from(max).unwrap_or(u64::MAX).saturating_add(1);
    let n = reader.take(limit).read_until(b'\n', buf).map_err(|e| match e.kind() {
        ErrorKind::TimedOut | ErrorKind::WouldBlock => "request line not completed in time; closing the connection".to_owned(),
        _ => e.to_string(),
    })?;
    if n == 0 {
        return Ok(false);
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        return Ok(true);
    }
    if buf.len() > max {
        return Err(format!("request line longer than {max} bytes; closing the connection"));
    }
    Ok(true)
}

/// A parsed line.
#[derive(Debug, PartialEq)]
pub enum Line {
    /// A request.
    Request {
        /// The client's id, echoed in the reply.
        id: Value,
        /// The method.
        method: String,
        /// Its parameters.
        params: Value,
    },
    /// An empty line.
    Blank,
    /// Not a request: reply with this and close.
    Reject(String),
}

/// Parses one request line.
pub fn parse_line(bytes: &[u8]) -> Line {
    let Ok(text) = std::str::from_utf8(bytes) else { return Line::Reject("not a control request (invalid UTF-8)".into()) };
    let text = text.trim();
    if text.is_empty() {
        return Line::Blank;
    }
    let first = text.split_once(' ').map_or(text, |(m, _)| m);
    if ["GET", "HEAD", "POST", "PUT", "DELETE", "OPTIONS", "PATCH", "CONNECT", "TRACE", "PRI"].contains(&first) {
        return Line::Reject("HTTP is not served here: this port takes JSON lines".into());
    }
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => return Line::Reject(format!("bad JSON ({e}): expected one request object per line")),
    };
    let Some(obj) = v.as_object() else { return Line::Reject("not a control request (expected a JSON object)".into()) };
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        return Line::Reject("not a control request (missing string `method`)".into());
    };
    Line::Request {
        id: obj.get("id").cloned().unwrap_or(Value::Null),
        method: method.to_owned(),
        params: obj.get("params").cloned().unwrap_or_else(|| json!({})),
    }
}

fn connection(stream: TcpStream, tx: &SyncSender<ControlRequest>, wake: &dyn Fn(), limits: Limits) {
    if stream.set_read_timeout(Some(limits.idle_timeout)).is_err() {
        return;
    }
    let Ok(read) = stream.try_clone() else { return };
    let mut out = stream;
    let mut reader = BufReader::new(read);
    let mut buf = Vec::new();
    loop {
        let reject = match read_line(&mut reader, &mut buf, limits.max_line) {
            Ok(false) => break,
            Err(_) if buf.is_empty() => break,
            Err(e) => e,
            Ok(true) => match parse_line(&buf) {
                Line::Blank => continue,
                Line::Reject(e) => e,
                Line::Request { id, method, params } => {
                    let (req, pending) = ControlRequest::new(method, params);
                    let mut r = match tx.try_send(req) {
                        Ok(()) => {
                            wake();
                            pending.wait(limits.reply_timeout)
                        }
                        Err(TrySendError::Full(_)) => json!({"ok": false, "error": "busy: too many queued requests", "state": "not-run"}),
                        Err(TrySendError::Disconnected(_)) => break,
                    };
                    if let Some(o) = r.as_object_mut() {
                        o.insert("id".into(), id);
                    }
                    if writeln!(out, "{r}").is_err() {
                        break;
                    }
                    continue;
                }
            },
        };
        let _ = writeln!(out, "{}", json!({"ok": false, "error": reject}));
        break;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    #[test]
    fn lines_are_bounded_and_crlf_stripped() {
        let mut buf = Vec::new();
        let mut input: &[u8] = b"abc\r\nde";
        assert_eq!(read_line(&mut input, &mut buf, 10), Ok(true));
        assert_eq!(buf, b"abc");
        assert_eq!(read_line(&mut input, &mut buf, 10), Ok(true));
        assert_eq!(buf, b"de");
        assert_eq!(read_line(&mut input, &mut buf, 10), Ok(false));
        let long = [b'x'; 100];
        let mut input: &[u8] = &long;
        assert!(read_line(&mut input, &mut buf, 10).is_err());
    }

    #[test]
    fn only_json_objects_with_a_method_are_requests() {
        assert!(matches!(parse_line(b"{\"id\":1,\"method\":\"ping\"}"), Line::Request { ref method, .. } if method == "ping"));
        assert_eq!(parse_line(b"   "), Line::Blank);
        for bad in [&b"GET / HTTP/1.1"[..], b"[1,2]", b"{\"id\":1}", b"\xff\xfe", b"{nope"] {
            assert!(matches!(parse_line(bad), Line::Reject(_)), "{bad:?}");
        }
    }

    #[test]
    fn a_request_round_trips_through_a_socket() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let rx = serve(listener, Limits::default(), Arc::new(|| {}));
        let ui = std::thread::spawn(move || {
            let req = rx.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(req.begin());
            let _ = req.reply.send(json!({"ok": true, "result": req.method}));
        });
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        writeln!(s, "{}", json!({"id": 7, "method": "ping"})).unwrap();
        let mut line = String::new();
        BufReader::new(s.try_clone().unwrap()).read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!((v["id"].clone(), v["result"].clone()), (json!(7), json!("ping")));
        ui.join().unwrap();
    }
}
