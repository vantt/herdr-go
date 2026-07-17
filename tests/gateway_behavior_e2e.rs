//! Behavioral e2e over a real server + WebSocket: observe mode fans out to
//! multiple watchers, and herdr going down is reflected honestly by the API
//! rather than crashing the gateway.

use std::sync::Arc;

use futures_util::StreamExt;
use herdctl::herdr::fake::FakeHerdr;
use herdctl::web::{router, AppState};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

async fn spawn() -> (String, FakeHerdr) {
    let fake = FakeHerdr::new();
    let state = AppState::new(
        Arc::new(fake.clone()),
        Arc::new(fake.clone()),
        Some("k".into()),
        16,
    );
    let app = router(state, std::path::Path::new("/nonexistent"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("127.0.0.1:{}", addr.port()), fake)
}

async fn login(addr: &str) -> String {
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{addr}/api/login"))
        .json(&serde_json::json!({ "token": "k" }))
        .send()
        .await
        .unwrap();
    res.headers()
        .get(reqwest::header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn open_ws(
    addr: &str,
    query: &str,
    cookie: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let mut req = format!("ws://{addr}/ws/terminal?{query}")
        .into_client_request()
        .unwrap();
    req.headers_mut().insert("Cookie", cookie.parse().unwrap());
    tokio_tungstenite::connect_async(req).await.unwrap().0
}

async fn next_frame(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        if let Message::Text(txt) = ws.next().await.unwrap().unwrap() {
            let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
            if v["type"] == "terminal.frame" {
                return v;
            }
        }
    }
}

#[tokio::test]
async fn observe_mode_fans_out_to_multiple_watchers() {
    let (addr, fake) = spawn().await;
    let cookie = login(&addr).await;

    let mut a = open_ws(&addr, "pane=pane-working&mode=observe", &cookie).await;
    let mut b = open_ws(&addr, "pane=pane-working&mode=observe", &cookie).await;

    // Both observers get the initial full redraw.
    assert!(next_frame(&mut a).await["full"].as_bool().unwrap());
    assert!(next_frame(&mut b).await["full"].as_bool().unwrap());

    // A single push reaches both (read-only fan-out).
    fake.push_output("pane-working", b"shared").await;
    let fa = next_frame(&mut a).await;
    let fb = next_frame(&mut b).await;
    assert_eq!(fa["bytes"], fb["bytes"]);
}

#[tokio::test]
async fn herdr_down_is_reported_not_crashed() {
    let (addr, fake) = spawn().await;
    let cookie = login(&addr).await;
    let client = reqwest::Client::new();

    // Healthy first.
    let up: serde_json::Value = client
        .get(format!("http://{addr}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(up["herdr_up"], true);

    // Take herdr down: health flips, agents 502s — the server stays alive.
    fake.set_available(false);
    let down: serde_json::Value = client
        .get(format!("http://{addr}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(down["herdr_up"], false);

    let agents = client
        .get(format!("http://{addr}/api/agents"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(agents.status().as_u16(), 502);

    // Recover: health returns to up.
    fake.set_available(true);
    let again: serde_json::Value = client
        .get(format!("http://{addr}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(again["herdr_up"], true);
}
