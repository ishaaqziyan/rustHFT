use axum::{Json, Router, routing::post};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
pub struct TvAlert {
    pub symbol: String,
    pub action: String, // "buy" or "sell"
    pub price: f64,
}

pub struct WebhookServer;

impl WebhookServer {
    pub async fn start() {
        let app = Router::new().route("/tv-alert", post(handle_tv_alert));

        let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
        println!("Webhook server listening on {}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

async fn handle_tv_alert(Json(payload): Json<TvAlert>) -> &'static str {
    println!("Received TradingView Alert: {:?}", payload);
    // Here you would forward this signal to the Engine via a channel
    "Alert received"
}
