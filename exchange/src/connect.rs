use crate::adapter::AdapterError;
use bytes::Bytes;
use fastwebsockets::FragmentCollector;
use http_body_util::Empty;
use hyper::{
    Request,
    header::{CONNECTION, UPGRADE},
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio_rustls::{
    TlsConnector,
    rustls::{ClientConfig, OwnedTrustAnchor},
};

/// وضعیت اتصال وب‌سوکت
#[allow(clippy::large_enum_variant)]
pub enum State {
    Disconnected,                                   // قطع شده
    Connected(FragmentCollector<TokioIo<Upgraded>>), // متصل شده
}

/// برقراری اتصال وب‌سوکت امن (WSS)
pub async fn connect_ws(
    domain: &str,
    url: &str,
) -> Result<
    fastwebsockets::FragmentCollector<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>,
    AdapterError,
> {
    // ۱. راه‌اندازی اتصال TCP
    let tcp_stream = setup_tcp(domain).await?;
    // ۲. ارتقا به لایه امن TLS
    let tls_stream = upgrade_to_tls(domain, tcp_stream).await?;

    // ۳. انجام دست‌دهی (Handshake) وب‌سوکت
    upgrade_to_websocket(domain, tls_stream, url).await
}

/// ساختار کمکی برای اجرای کارهای ناهمگام در پس‌زمینه
struct SpawnExecutor;

impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
    Fut: std::future::Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        tokio::task::spawn(fut);
    }
}

/// راه‌اندازی اتصال TCP به دامنه مورد نظر روی پورت ۴۴۳
async fn setup_tcp(domain: &str) -> Result<TcpStream, AdapterError> {
    let addr = format!("{domain}:443");
    TcpStream::connect(&addr)
        .await
        .map_err(|e| AdapterError::WebsocketError(e.to_string()))
}

/// ایجاد تنظیمات و کانکتور TLS با استفاده از گواهی‌های ریشه سیستم
fn tls_connector() -> Result<TlsConnector, AdapterError> {
    let mut root_store = tokio_rustls::rustls::RootCertStore::empty();

    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(TlsConnector::from(std::sync::Arc::new(config)))
}

/// ارتقای اتصال TCP به یک اتصال امن TLS
async fn upgrade_to_tls(
    domain: &str,
    tcp_stream: TcpStream,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, AdapterError> {
    let domain: tokio_rustls::rustls::ServerName =
        tokio_rustls::rustls::ServerName::try_from(domain)
            .map_err(|_| AdapterError::ParseError("invalid dnsname".to_string()))?;

    tls_connector()?
        .connect(domain, tcp_stream)
        .await
        .map_err(|e| AdapterError::WebsocketError(e.to_string()))
}

/// انجام دست‌دهی وب‌سوکت روی لایه امن TLS
async fn upgrade_to_websocket(
    domain: &str,
    tls_stream: tokio_rustls::client::TlsStream<TcpStream>,
    url: &str,
) -> Result<FragmentCollector<TokioIo<Upgraded>>, AdapterError> {
    let req: Request<Empty<Bytes>> = Request::builder()
        .method("GET")
        .uri(url)
        .header("Host", domain)
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "upgrade")
        .header(
            "Sec-WebSocket-Key",
            fastwebsockets::handshake::generate_key(),
        )
        .header("Sec-WebSocket-Version", "13")
        .body(Empty::<Bytes>::new())
        .map_err(|e| AdapterError::WebsocketError(e.to_string()))?;

    let (ws, _) = fastwebsockets::handshake::client(&SpawnExecutor, req, tls_stream)
        .await
        .map_err(|e| AdapterError::WebsocketError(e.to_string()))?;

    Ok(FragmentCollector::new(ws))
}
