//! A minimal HTTP/1.0 client for Syncthing's REST API on the loopback.
//!
//! Plain HTTP over a `TcpStream`, no new crate: the API is on `127.0.0.1`
//! only, every request carries the API key, and an HTTP/1.0 request makes the
//! server answer without chunking and close the connection, so the body is
//! "everything after the headers". A chunked answer is still decoded, in case
//! a proxy-like server sends one. Time and size are bounded.

use super::json::Json;
use std::io::{self, Error, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Longest answer read.
pub const MAX_REPLY: u64 = 512 * 1024;

/// Time allowed to connect, and for each read or write.
const TIMEOUT: Duration = Duration::from_secs(10);

/// One answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The HTTP status.
    pub status: u16,
    /// The body, as text.
    pub body: String,
}

/// A connection recipe for one Syncthing: where, and the key.
#[derive(Clone, Debug)]
pub struct Client {
    addr: SocketAddr,
    key: String,
}

impl Client {
    /// A client for the API on `127.0.0.1:port`.
    #[must_use]
    pub fn new(port: u16, key: impl Into<String>) -> Self {
        Self {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
            key: key.into(),
        }
    }

    /// Sends one request and reads the whole answer.
    ///
    /// # Errors
    /// The connection failed or timed out, the answer was too long, or it
    /// was not HTTP.
    pub fn request(&self, method: &str, path: &str, body: Option<&str>) -> io::Result<Reply> {
        let mut s = TcpStream::connect_timeout(&self.addr, TIMEOUT)?;
        s.set_read_timeout(Some(TIMEOUT))?;
        s.set_write_timeout(Some(TIMEOUT))?;
        let body = body.unwrap_or("");
        let head = format!(
            "{method} {path} HTTP/1.0\r\nHost: {}\r\nX-API-Key: {}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.addr,
            self.key,
            body.len()
        );
        s.write_all(head.as_bytes())?;
        s.write_all(body.as_bytes())?;
        let mut raw = Vec::new();
        s.take(MAX_REPLY + 1).read_to_end(&mut raw)?;
        if raw.len() as u64 > MAX_REPLY {
            return Err(Error::new(ErrorKind::InvalidData, "answer too long"));
        }
        parse_reply(&raw)
    }

    /// `GET path`, read as JSON.
    ///
    /// # Errors
    /// Anything [`Client::request`] fails on, a status other than 200, or a
    /// body that is not JSON.
    pub fn get_json(&self, path: &str) -> io::Result<Json> {
        let r = self.request("GET", path, None)?;
        if r.status != 200 {
            return Err(Error::other(format!("GET {path}: HTTP {}", r.status)));
        }
        Json::parse(&r.body).ok_or_else(|| Error::new(ErrorKind::InvalidData, "not JSON"))
    }

    /// Sends `body` with `method` and wants a 2xx back.
    ///
    /// # Errors
    /// Anything [`Client::request`] fails on, or a status that is not 2xx
    /// (with the first line of Syncthing's text).
    pub fn send(&self, method: &str, path: &str, body: &str) -> io::Result<()> {
        let r = self.request(method, path, Some(body))?;
        if (200..300).contains(&r.status) {
            Ok(())
        } else {
            let why = r.body.lines().next().unwrap_or("");
            Err(Error::other(format!(
                "{method} {path}: HTTP {} {why}",
                r.status
            )))
        }
    }
}

fn bad(why: &str) -> Error {
    Error::new(ErrorKind::InvalidData, why.to_owned())
}

/// Splits raw bytes into status and body.
fn parse_reply(raw: &[u8]) -> io::Result<Reply> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| bad("no HTTP headers"))?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.lines();
    let status = lines
        .next()
        .filter(|l| l.starts_with("HTTP/"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| bad("bad status line"))?;
    let chunked = lines.any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let body = &raw[split + 4..];
    let body = if chunked {
        unchunk(body)?
    } else {
        body.to_vec()
    };
    Ok(Reply {
        status,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn unchunk(mut b: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let eol = b
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| bad("bad chunk"))?;
        let size = std::str::from_utf8(&b[..eol])
            .ok()
            .and_then(|s| usize::from_str_radix(s.split(';').next().unwrap_or("").trim(), 16).ok())
            .ok_or_else(|| bad("bad chunk size"))?;
        b = &b[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if b.len() < size {
            return Err(bad("short chunk"));
        }
        out.extend_from_slice(&b[..size]);
        b = b.get(size + 2..).unwrap_or(&[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_chunked_answers_decode() {
        let r = parse_reply(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").unwrap();
        assert_eq!((r.status, r.body.as_str()), (200, "{}"));
        let c = parse_reply(
            b"HTTP/1.1 404 Not Found\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n",
        )
        .unwrap();
        assert_eq!((c.status, c.body.as_str()), (404, "abcde"));
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        for raw in [
            &b""[..],
            b"HTTP/1.1 200 OK",
            b"junk\r\n\r\nbody",
            b"HTTP/1.1 x\r\n\r\n",
        ] {
            assert!(parse_reply(raw).is_err());
        }
        assert!(
            parse_reply(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n").is_err()
        );
        assert!(
            parse_reply(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n9\r\nab").is_err()
        );
    }
}
