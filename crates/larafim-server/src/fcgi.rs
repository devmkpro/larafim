//! Cliente FastCGI mínimo (uma requisição por conexão), suficiente para
//! conversar com o `php-cgi.exe`. Especificação: FastCGI 1.0.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const VERSION: u8 = 1;
const BEGIN_REQUEST: u8 = 1;
const END_REQUEST: u8 = 3;
const PARAMS: u8 = 4;
const STDIN: u8 = 5;
const STDOUT: u8 = 6;
const STDERR: u8 = 7;
const RESPONDER: u16 = 1;
const REQUEST_ID: u16 = 1;
const MAX_CONTENT: usize = 65_535;

#[derive(Debug, Default)]
pub struct FcgiResponse {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Cabeçalho de 8 bytes de um registro.
fn header(kind: u8, len: usize) -> [u8; 8] {
    let id = REQUEST_ID.to_be_bytes();
    let len = (len as u16).to_be_bytes();
    [VERSION, kind, id[0], id[1], len[0], len[1], 0, 0]
}

/// Escreve o conteúdo em registros de até 64 KB e um registro vazio no fim.
fn push_stream(out: &mut Vec<u8>, kind: u8, data: &[u8]) {
    for chunk in data.chunks(MAX_CONTENT) {
        out.extend_from_slice(&header(kind, chunk.len()));
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&header(kind, 0));
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    if len < 128 {
        out.push(len as u8);
    } else {
        out.extend_from_slice(&((len as u32) | 0x8000_0000).to_be_bytes());
    }
}

/// Codifica os pares nome/valor do registro PARAMS.
pub fn encode_params(params: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in params {
        push_len(&mut out, name.len());
        push_len(&mut out, value.len());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    out
}

/// Monta todos os bytes de uma requisição.
pub fn encode_request(params: &[(String, String)], stdin: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stdin.len() + 1024);
    out.extend_from_slice(&header(BEGIN_REQUEST, 8));
    out.extend_from_slice(&RESPONDER.to_be_bytes());
    out.extend_from_slice(&[0; 6]); // flags = 0 (fecha a conexão no fim) + reservado
    push_stream(&mut out, PARAMS, &encode_params(params));
    push_stream(&mut out, STDIN, stdin);
    out
}

/// Envia a requisição e lê a resposta inteira.
pub async fn execute(
    addr: SocketAddr,
    params: &[(String, String)],
    stdin: &[u8],
    timeout: Duration,
) -> std::io::Result<FcgiResponse> {
    tokio::time::timeout(timeout, async {
        let mut stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        stream.write_all(&encode_request(params, stdin)).await?;
        read_response(&mut stream).await
    })
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, larafim_core::t!("o PHP demorou demais para responder")))?
}

async fn read_response<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<FcgiResponse> {
    let mut resp = FcgiResponse::default();
    loop {
        let mut h = [0u8; 8];
        reader.read_exact(&mut h).await?;
        let kind = h[1];
        let len = u16::from_be_bytes([h[4], h[5]]) as usize;
        let pad = h[6] as usize;
        let mut body = vec![0u8; len + pad];
        reader.read_exact(&mut body).await?;
        body.truncate(len);
        match kind {
            STDOUT => resp.stdout.extend_from_slice(&body),
            STDERR => resp.stderr.extend_from_slice(&body),
            END_REQUEST => return Ok(resp),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_lengths() {
        let short = encode_params(&[("A".into(), "b".into())]);
        assert_eq!(short, [1, 1, b'A', b'b']);
        let long_value = "x".repeat(200);
        let long = encode_params(&[("K".into(), long_value.clone())]);
        assert_eq!(&long[..5], &[1, 0x80, 0, 0, 200]);
        assert_eq!(long.len(), 1 + 4 + 1 + 200);
    }

    #[test]
    fn big_stdin_is_split() {
        let body = vec![7u8; 70_000];
        let bytes = encode_request(&[], &body);
        // BEGIN(16) + PARAMS vazio(8) + STDIN 65535 + STDIN 4465 + STDIN vazio
        assert_eq!(bytes.len(), 16 + 8 + (8 + 65_535) + (8 + 4_465) + 8);
    }

    /// Servidor FastCGI falso: confere o pedido e responde.
    #[tokio::test]
    async fn roundtrip_with_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // lê até o STDIN vazio
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
                if got.ends_with(&header(STDIN, 0)) {
                    break;
                }
            }
            let mut out = Vec::new();
            let mut rec = |kind: u8, data: &[u8]| {
                out.extend_from_slice(&header(kind, data.len()));
                out.extend_from_slice(data);
            };
            rec(STDOUT, b"Status: 201 Created\r\nContent-type: text/plain\r\n\r\n");
            rec(STDOUT, b"ola");
            rec(STDERR, b"aviso");
            rec(END_REQUEST, &[0; 8]);
            sock.write_all(&out).await.unwrap();
        });
        let resp = execute(addr, &[("REQUEST_METHOD".into(), "GET".into())], b"", Duration::from_secs(5))
            .await
            .unwrap();
        assert!(resp.stdout.ends_with(b"\r\n\r\nola"));
        assert_eq!(resp.stderr, b"aviso");
    }
}
