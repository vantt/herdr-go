//! End-to-end proof of the Tier 2 relay: a real axum server backed by the Fake
//! herdr, driven over a real WebSocket exactly as the browser would — login →
//! open control WS → receive the full redraw frame → type input → see it echoed
//! back as a diff frame. This is the app's main axis, proven without a live herdr.

use std::sync::Arc;

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use herdctl::herdr::fake::FakeHerdr;
use herdctl::web::{router, AppState};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_server() -> (String, FakeHerdr) {
    let fake = FakeHerdr::new();
    let control = Arc::new(fake.clone());
    let stream = Arc::new(fake.clone());
    let state = AppState::new(control, stream, Some("e2e-secret".into()), 16);
    // No static dir needed; router serves the API + WS.
    let app = router(state, std::path::Path::new("/nonexistent-static"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("127.0.0.1:{}", addr.port()), fake)
}

#[tokio::test]
async fn tier2_control_round_trips_over_real_websocket() {
    let (addr, _fake) = spawn_server().await;

    // 1. Login over HTTP to obtain the session cookie.
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{addr}/api/login"))
        .json(&serde_json::json!({ "token": "e2e-secret" }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    let set_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let cookie = set_cookie.split(';').next().unwrap().to_string();

    // 2. Open a control-mode WebSocket to the working pane, carrying the cookie.
    let url = format!("ws://{addr}/ws/terminal?pane=pane-working&mode=control&takeover=true");
    let mut req = url.into_client_request().unwrap();
    req.headers_mut().insert("Cookie", cookie.parse().unwrap());
    let (mut ws, _resp) = tokio_tungstenite::connect_async(req).await.unwrap();

    // 3. First frame is the full redraw.
    let first = next_frame(&mut ws).await;
    assert!(
        first["full"].as_bool().unwrap(),
        "first frame is a full redraw"
    );
    assert_eq!(first["type"], "terminal.frame");

    // 4. Type input; expect it echoed back as a diff frame with our bytes.
    ws.send(Message::Text(
        serde_json::json!({ "t": "input", "data": "whoami\n" }).to_string(),
    ))
    .await
    .unwrap();

    let echoed = next_frame(&mut ws).await;
    assert!(!echoed["full"].as_bool().unwrap());
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(echoed["bytes"].as_str().unwrap())
        .unwrap();
    assert_eq!(decoded, b"whoami\n");
}

#[tokio::test]
async fn unauthenticated_ws_is_refused() {
    let (addr, _fake) = spawn_server().await;
    let url = format!("ws://{addr}/ws/terminal?pane=pane-working&mode=observe");
    // No cookie → the AuthSession extractor rejects with 404 before upgrade.
    let result = tokio_tungstenite::connect_async(url).await;
    assert!(result.is_err(), "unauthenticated WS upgrade must fail");
}

/// Read WS messages until a terminal frame arrives, returning it parsed.
async fn next_frame(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        let msg = ws.next().await.unwrap().unwrap();
        if let Message::Text(txt) = msg {
            let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
            if v["type"] == "terminal.frame" {
                return v;
            }
        }
    }
}
