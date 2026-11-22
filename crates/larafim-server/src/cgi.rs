//! Lê a saída CGI do PHP (cabeçalhos + corpo) e transforma em resposta HTTP.

#[derive(Debug, PartialEq, Eq)]
pub struct CgiResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Separa cabeçalhos e corpo. Aceita `\r\n\r\n` ou `\n\n`. O cabeçalho
/// `Status:` vira o código HTTP; sem `Location` nem `Status`, é 200.
pub fn parse(output: &[u8]) -> CgiResponse {
    let (head, body) = split_head(output);
    let mut status = None;
    let mut headers = Vec::new();
    for line in String::from_utf8_lossy(head).lines() {
        let Some((name, value)) = line.split_once(':') else { continue };
        let (name, value) = (name.trim(), value.trim());
        if name.eq_ignore_ascii_case("status") {
            status = value.split_whitespace().next().and_then(|c| c.parse().ok());
        } else if !name.is_empty() {
            headers.push((name.to_string(), value.to_string()));
        }
    }
    let has_location = headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("location"));
    let status = status.unwrap_or(if has_location { 302 } else { 200 });
    if !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("content-type")) {
        headers.push(("Content-Type".into(), "text/html; charset=UTF-8".into()));
    }
    CgiResponse { status, headers, body: body.to_vec() }
}

fn split_head(output: &[u8]) -> (&[u8], &[u8]) {
    for (sep, len) in [(&b"\r\n\r\n"[..], 4), (&b"\n\n"[..], 2)] {
        if let Some(pos) = output.windows(len).position(|w| w == sep) {
            return (&output[..pos], &output[pos + len..]);
        }
    }
    (&[], output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_headers_and_cookies() {
        let out = b"Status: 404 Not Found\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nContent-Type: application/json\r\n\r\n{}";
        let r = parse(out);
        assert_eq!(r.status, 404);
        assert_eq!(r.body, b"{}");
        let cookies: Vec<_> = r.headers.iter().filter(|(n, _)| n == "Set-Cookie").collect();
        assert_eq!(cookies.len(), 2);
    }

    #[test]
    fn redirect_without_status_is_302_and_default_type() {
        let r = parse(b"Location: /login\n\n");
        assert_eq!(r.status, 302);
        assert!(r.headers.iter().any(|(n, v)| n == "Content-Type" && v.starts_with("text/html")));
    }

    #[test]
    fn plain_output() {
        let r = parse(b"X-Powered-By: PHP\r\n\r\n<h1>oi</h1>");
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"<h1>oi</h1>");
    }
}
