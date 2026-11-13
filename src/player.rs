//! YouTube refuses embeds from pages that send no HTTP Referer ("Error 153"), and a page loaded from the
//! app's `tauri://` scheme sends none. This serves one static player page on 127.0.0.1, so the lesson
//! iframe is `http://127.0.0.1:<port>/player.html` and YouTube sees that page as the referrer. The server
//! holds no app state and exposes no commands; anything else on the machine can only read these two files.

use std::sync::Arc;
use tiny_http::{Header, Request, Response, Server};

const PAGE: &str = include_str!("player/player.html");
const SCRIPT: &str = include_str!("player/player.js");
// Pages that may frame the player: the app on Linux/macOS, the app on Windows, and the Vite dev server.
const PAGE_CSP: &str = "default-src 'none'; script-src 'self' https://www.youtube.com; \
frame-src https://www.youtube.com https://www.youtube-nocookie.com; style-src 'unsafe-inline'; \
frame-ancestors tauri://localhost http://tauri.localhost https://tauri.localhost http://localhost:5173";
/// The `frame-src` source in `tauri.conf.json` that `app_csp` narrows to the player's port.
pub const CSP_PLACEHOLDER: &str = "frame-src http://127.0.0.1;";

/// Starts the player server on an OS-chosen 127.0.0.1 port and returns its origin.
pub fn start() -> std::io::Result<String> {
    let server = Arc::new(Server::http("127.0.0.1:0").map_err(std::io::Error::other)?);
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or_else(|| std::io::Error::other("player server has no port"))?;
    let origin = format!("http://127.0.0.1:{port}");
    let host = format!("127.0.0.1:{port}");
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let _ = answer(request, &host);
        }
    });
    Ok(origin)
}

fn answer(request: Request, host: &str) -> std::io::Result<()> {
    let (status, body, kind) = route(
        request.method().as_str(),
        request.url(),
        request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Host"))
            .map(|h| h.value.as_str()),
        host,
    );
    let mut response = Response::from_string(body).with_status_code(status);
    for (name, value) in [
        ("Content-Type", kind),
        ("Content-Security-Policy", PAGE_CSP),
        ("Referrer-Policy", "strict-origin-when-cross-origin"),
        ("X-Content-Type-Options", "nosniff"),
        ("Cache-Control", "no-store"),
    ] {
        response.add_header(Header::from_bytes(name, value).expect("static header"));
    }
    request.respond(response)
}

/// Picks the response; a wrong Host (DNS rebinding) or any other path gets nothing.
fn route(method: &str, url: &str, got_host: Option<&str>, host: &str) -> (u16, &'static str, &'static str) {
    if got_host != Some(host) {
        return (403, "", "text/plain");
    }
    if method != "GET" {
        return (405, "", "text/plain");
    }
    match url.split('?').next() {
        Some("/player.html") => (200, PAGE, "text/html; charset=utf-8"),
        Some("/player.js") => (200, SCRIPT, "text/javascript; charset=utf-8"),
        _ => (404, "", "text/plain"),
    }
}

/// The app CSP with the placeholder `frame-src` narrowed to exactly the player's origin.
pub fn app_csp(csp: &str, origin: &str) -> Option<String> {
    csp.contains(CSP_PLACEHOLDER)
        .then(|| csp.replace(CSP_PLACEHOLDER, &format!("frame-src {origin};")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_only_the_player_to_its_own_host() {
        let h = "127.0.0.1:4000";
        assert_eq!(route("GET", "/player.html?v=abc", Some(h), h).0, 200);
        assert_eq!(route("GET", "/player.js", Some(h), h).2, "text/javascript; charset=utf-8");
        assert_eq!(route("GET", "/", Some(h), h).0, 404);
        assert_eq!(route("GET", "/../Cargo.toml", Some(h), h).0, 404);
        assert_eq!(route("POST", "/player.html", Some(h), h).0, 405);
        assert_eq!(route("GET", "/player.html", Some("evil.example:4000"), h).0, 403);
        assert_eq!(route("GET", "/player.html", None, h).0, 403);
    }

    #[test]
    fn app_csp_allows_only_the_player_frame() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = conf["app"]["security"]["csp"].as_str().unwrap();
        let out = app_csp(csp, "http://127.0.0.1:4000").expect("placeholder in tauri.conf.json");
        let frame = out.split(';').find(|d| d.trim().starts_with("frame-src")).unwrap();
        assert_eq!(frame.trim(), "frame-src http://127.0.0.1:4000");
        assert_eq!(app_csp("default-src 'self'", "http://127.0.0.1:4000"), None);
    }

    #[test]
    fn server_answers_over_loopback() {
        use std::io::{Read, Write};
        let origin = start().unwrap();
        let addr = origin.trim_start_matches("http://");
        let mut s = std::net::TcpStream::connect(addr).unwrap();
        write!(s, "GET /player.html?v=CMv1zlZhb4Q HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").unwrap();
        let mut got = String::new();
        s.read_to_string(&mut got).unwrap();
        assert!(got.starts_with("HTTP/1.1 200"), "{got}");
        assert!(got.contains("frame-ancestors tauri://localhost"));
        assert!(got.contains("player.js"));
    }
}
