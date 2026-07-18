//! End-to-end over a real axum server backed by the Fake herdr: log in, list
//! agents, poll a pane's screen, post a reply, and see it land — the snapshot +
//! reply model, proven without a live herdr. Also: herdr going down is reported,
//! not crashed.

use std::sync::Arc;

use herdctl::herdr::fake::FakeHerdr;
use herdctl::web::{router, AppState};

async fn spawn() -> (String, FakeHerdr) {
    let fake = FakeHerdr::new();
    let state = AppState::new(Arc::new(fake.clone()), Some("k".into()), 16);
    let app = router(state, std::path::Path::new("/nonexistent"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("127.0.0.1:{}", addr.port()), fake)
}

async fn login(client: &reqwest::Client, addr: &str) -> String {
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

#[tokio::test]
async fn observe_then_reply_round_trips() {
    let (addr, _fake) = spawn().await;
    let client = reqwest::Client::new();
    let cookie = login(&client, &addr).await;

    // Switcher lists the seeded agents.
    let agents: Vec<serde_json::Value> = client
        .get(format!("http://{addr}/api/agents"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agents.len(), 4);
    let pane = agents[0]["pane_id"].as_str().unwrap().to_string();

    // Poll that pane's screen.
    let screen: serde_json::Value = client
        .get(format!("http://{addr}/api/panes/{pane}/screen"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(screen["text"]
        .as_str()
        .unwrap()
        .contains("Building the parser"));
    let rev0 = screen["revision"].as_u64().unwrap();

    // Post a reply; it lands (screen changes, revision bumps).
    let ok = client
        .post(format!("http://{addr}/api/panes/{pane}/input"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "text": "ship it" }))
        .send()
        .await
        .unwrap();
    assert!(ok.status().is_success());

    let after: serde_json::Value = client
        .get(format!("http://{addr}/api/panes/{pane}/screen"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(after["text"].as_str().unwrap().contains("ship it"));
    assert!(after["revision"].as_u64().unwrap() > rev0);
}

#[tokio::test]
async fn unauth_screen_is_opaque_404() {
    let (addr, _fake) = spawn().await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://{addr}/api/panes/w1:p1/screen"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 404);
}

#[tokio::test]
async fn herdr_down_is_reported_not_crashed() {
    let (addr, fake) = spawn().await;
    let client = reqwest::Client::new();

    let up: serde_json::Value = client
        .get(format!("http://{addr}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(up["herdr_up"], true);

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
}
