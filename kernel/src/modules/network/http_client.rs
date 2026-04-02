/// HTTP/1.1 Client
///
/// Provides outbound HTTP request building and response parsing for use by
/// kernel subsystems (e.g. health-check beacons, telemetry export, OTA update).
///
/// # Limitations
/// * Operates over the kernel's `TcpStream` abstraction (loopback / smoltcp).
/// * No persistent TLS — pair with the `https` module for encrypted channels.
/// * Response body is limited to `MAX_RESPONSE_BODY` bytes; larger bodies are
///   silently truncated.
/// * Chunked transfer encoding is decoded but chunk extensions are ignored.

#[cfg(feature = "network_http")]
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum response body size to collect (16 KiB).
pub const MAX_RESPONSE_BODY: usize = 16 * 1024;
/// Maximum number of response headers to parse.
pub const MAX_RESPONSE_HEADERS: usize = 64;
/// Maximum line length in the response headers.
const MAX_HEADER_LINE: usize = 8192;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Supported HTTP request methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Options,
}

impl HttpMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
        }
    }
}

/// A parsed HTTP/1.1 response.
#[derive(Debug, Clone)]
pub struct HttpClientResponse {
    /// HTTP status code (e.g. 200, 404).
    pub status: u16,
    /// Reason phrase (e.g. "OK").
    pub reason: String,
    /// Response headers as (name, value) pairs (names are lower-cased).
    pub headers: Vec<(String, String)>,
    /// Response body bytes (may be truncated at `MAX_RESPONSE_BODY`).
    pub body: Vec<u8>,
    /// Whether the response used chunked transfer encoding.
    pub chunked: bool,
}

impl HttpClientResponse {
    /// Retrieve the value of the first matching (case-insensitive) header.
    pub fn header(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.as_str() == lower.as_str())
            .map(|(_, v)| v.as_str())
    }

    /// Retrieve `content-length` as a parsed integer.
    pub fn content_length(&self) -> Option<usize> {
        self.header("content-length")
            .and_then(|v| v.trim().parse::<usize>().ok())
    }
}

// ── Request builder ───────────────────────────────────────────────────────────

/// Builds a raw HTTP/1.1 request byte string.
///
/// # Example
/// ```
/// let req = HttpRequestBuilder::new(HttpMethod::Get, "/api/status")
///     .host("10.0.2.2")
///     .header("accept", "application/json")
///     .build();
/// ```
pub struct HttpRequestBuilder {
    method: HttpMethod,
    path: String,
    host: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    version: &'static str,
}

impl HttpRequestBuilder {
    /// Create a new builder for `method` on `path`.
    pub fn new(method: HttpMethod, path: &str) -> Self {
        let path = if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        };
        Self {
            method,
            path,
            host: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
            version: "HTTP/1.1",
        }
    }

    /// Set the `Host` header.
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    /// Append a custom header (name and value).
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_lowercase(), value.to_string()));
        self
    }

    /// Attach a request body (sets `Content-Length` automatically).
    pub fn body(mut self, data: Vec<u8>) -> Self {
        self.body = data;
        self
    }

    /// Attach a JSON body and set `Content-Type: application/json`.
    pub fn json_body(mut self, json: &str) -> Self {
        self.headers
            .push(("content-type".to_string(), "application/json".to_string()));
        self.body = json.as_bytes().to_vec();
        self
    }

    /// Serialise the request to raw bytes ready to send over a TCP stream.
    pub fn build(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256 + self.body.len());

        // Request line
        let line = format!(
            "{} {} {}\r\n",
            self.method.as_str(),
            self.path,
            self.version
        );
        out.extend_from_slice(line.as_bytes());

        // Mandatory Host header
        if !self.host.is_empty() {
            let h = format!("Host: {}\r\n", self.host);
            out.extend_from_slice(h.as_bytes());
        }

        // Standard headers
        out.extend_from_slice(b"Connection: keep-alive\r\n");
        out.extend_from_slice(b"User-Agent: AetherCore-Kernel/0.2\r\n");
        out.extend_from_slice(b"Accept: */*\r\n");

        // Content length when body present
        if !self.body.is_empty() {
            let cl = format!("Content-Length: {}\r\n", self.body.len());
            out.extend_from_slice(cl.as_bytes());
        }

        // Custom headers
        for (name, value) in &self.headers {
            let h = format!("{}: {}\r\n", name, value);
            out.extend_from_slice(h.as_bytes());
        }

        // Header/body delimiter
        out.extend_from_slice(b"\r\n");

        // Body
        if !self.body.is_empty() {
            out.extend_from_slice(&self.body);
        }

        out
    }
}

// ── Response parser ───────────────────────────────────────────────────────────

/// Parse a raw HTTP/1.1 response from a byte buffer.
///
/// Returns `Err` if the response is malformed.
pub fn parse_response(raw: &[u8]) -> Result<HttpClientResponse, &'static str> {
    // Split header block from body at double CRLF.
    let split = find_header_end(raw).ok_or("http response: no header/body boundary")?;
    let header_block = &raw[..split];
    let body_start = split + 4; // skip "\r\n\r\n"
    let body_raw = if body_start <= raw.len() {
        &raw[body_start..]
    } else {
        &[]
    };

    // Parse status line ("HTTP/1.1 200 OK\r\n")
    let mut lines = split_lines(header_block);
    let status_line = lines.next().ok_or("http response: missing status line")?;
    let (status, reason) = parse_status_line(status_line)?;

    // Parse headers
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let name = k.trim().to_lowercase();
            let value = v.trim().to_string();
            if headers.len() < MAX_RESPONSE_HEADERS {
                headers.push((name, value));
            }
        }
    }

    // Determine if chunked
    let chunked = headers
        .iter()
        .any(|(k, v)| k == "transfer-encoding" && v.contains("chunked"));

    // Decode body
    let body = if chunked {
        decode_chunked(body_raw)
    } else {
        let limit = MAX_RESPONSE_BODY.min(body_raw.len());
        body_raw[..limit].to_vec()
    };

    Ok(HttpClientResponse {
        status,
        reason,
        headers,
        body,
        chunked,
    })
}

// ── Chunked transfer decoding ─────────────────────────────────────────────────

/// Decode a `Transfer-Encoding: chunked` body.  Ignores chunk extensions.
fn decode_chunked(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;

    while pos < raw.len() && out.len() < MAX_RESPONSE_BODY {
        // Read chunk-size line (hex digits followed by optional extension, then CRLF)
        let line_end = match find_crlf(&raw[pos..]) {
            Some(e) => pos + e,
            None => break,
        };
        let size_str = core::str::from_utf8(&raw[pos..line_end]).unwrap_or("0");
        // Strip optional chunk extension (everything after ';')
        let size_str = size_str.split(';').next().unwrap_or("0").trim();
        let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);

        pos = line_end + 2; // advance past CRLF
        if chunk_size == 0 {
            break;
        } // terminal chunk

        let end = (pos + chunk_size).min(raw.len());
        let take = (end - pos).min(MAX_RESPONSE_BODY - out.len());
        out.extend_from_slice(&raw[pos..pos + take]);

        pos = end;
        // Skip trailing CRLF after chunk data
        if pos + 1 < raw.len() && raw[pos] == b'\r' && raw[pos + 1] == b'\n' {
            pos += 2;
        }
    }
    out
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

fn split_lines(buf: &[u8]) -> impl Iterator<Item = &str> {
    struct Lines<'a> {
        buf: &'a [u8],
        pos: usize,
    }
    impl<'a> Iterator for Lines<'a> {
        type Item = &'a str;
        fn next(&mut self) -> Option<&'a str> {
            if self.pos >= self.buf.len() {
                return None;
            }
            let rest = &self.buf[self.pos..];
            let eol = rest
                .windows(2)
                .position(|w| w == b"\r\n")
                .unwrap_or(rest.len());
            let line = core::str::from_utf8(&rest[..eol]).unwrap_or("");
            self.pos += eol + 2;
            Some(line)
        }
    }
    Lines { buf, pos: 0 }
}

fn parse_status_line(line: &str) -> Result<(u16, String), &'static str> {
    // "HTTP/1.1 200 OK"  or  "HTTP/1.0 404 Not Found"
    let mut parts = line.splitn(3, ' ');
    let _version = parts.next().ok_or("http: missing version")?;
    let code_str = parts.next().ok_or("http: missing status code")?;
    let reason = parts.next().unwrap_or("").to_string();
    let status = code_str
        .parse::<u16>()
        .map_err(|_| "http: invalid status code")?;
    Ok((status, reason))
}

// ── Stats ─────────────────────────────────────────────────────────────────────

use core::sync::atomic::{AtomicU64, Ordering};

static CLIENT_REQUESTS_BUILT: AtomicU64 = AtomicU64::new(0);
static CLIENT_RESPONSES_PARSED: AtomicU64 = AtomicU64::new(0);
static CLIENT_PARSE_ERRORS: AtomicU64 = AtomicU64::new(0);
static CLIENT_CHUNKED_DECODED: AtomicU64 = AtomicU64::new(0);

/// Record that a request was serialised (called by the caller after `build()`).
pub fn record_request_built() {
    CLIENT_REQUESTS_BUILT.fetch_add(1, Ordering::Relaxed);
}

/// Record the result of `parse_response()`.
pub fn record_parse_result(ok: bool, is_chunked: bool) {
    if ok {
        CLIENT_RESPONSES_PARSED.fetch_add(1, Ordering::Relaxed);
        if is_chunked {
            CLIENT_CHUNKED_DECODED.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        CLIENT_PARSE_ERRORS.fetch_add(1, Ordering::Relaxed);
    }
}

/// HTTP client telemetry.
#[derive(Debug, Clone, Copy)]
pub struct HttpClientStats {
    pub requests_built: u64,
    pub responses_parsed: u64,
    pub parse_errors: u64,
    pub chunked_decoded: u64,
}

pub fn stats() -> HttpClientStats {
    HttpClientStats {
        requests_built: CLIENT_REQUESTS_BUILT.load(Ordering::Relaxed),
        responses_parsed: CLIENT_RESPONSES_PARSED.load(Ordering::Relaxed),
        parse_errors: CLIENT_PARSE_ERRORS.load(Ordering::Relaxed),
        chunked_decoded: CLIENT_CHUNKED_DECODED.load(Ordering::Relaxed),
    }
}
