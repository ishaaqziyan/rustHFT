use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hft_core::{MarketEvent, OrderBookLevel, OrderBookSnapshot, Tick};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub async fn start_alpaca_ws(
    api_key: String,
    api_secret: String,
    symbols: Vec<String>,
    tx: mpsc::Sender<MarketEvent>,
) {
    let url = "wss://stream.data.alpaca.markets/v2/iex";
    println!("Connecting to Alpaca WebSocket: {}", url);

    match connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            println!("Connected to Alpaca WebSocket!");

            // 1. Authenticate
            let auth_msg = json!({
                "action": "auth",
                "key": api_key,
                "secret": api_secret
            });
            let _ = ws_stream
                .send(Message::Text(auth_msg.to_string().into()))
                .await;

            // 2. Subscribe
            let sub_msg = json!({
                "action": "subscribe",
                "quotes": symbols
            });
            let _ = ws_stream
                .send(Message::Text(sub_msg.to_string().into()))
                .await;

            // 3. Listen for messages
            while let Some(msg) = ws_stream.next().await {
                if let Ok(Message::Text(text)) = msg
                    && let Ok(data) = serde_json::from_str::<serde_json::Value>(&text)
                        && let Some(arr) = data.as_array() {
                            for item in arr {
                                if let Some(msg_type) = item.get("T").and_then(|v| v.as_str()) {
                                    if msg_type == "q" {
                                        // Quote message
                                        if let (
                                            Some(sym),
                                            Some(bp),
                                            Some(ap),
                                            Some(bs),
                                            Some(as_size),
                                        ) = (
                                            item.get("S").and_then(|v| v.as_str()),
                                            item.get("bp").and_then(|v| v.as_f64()),
                                            item.get("ap").and_then(|v| v.as_f64()),
                                            item.get("bs").and_then(|v| v.as_f64()),
                                            item.get("as").and_then(|v| v.as_f64()),
                                        ) {
                                            let mid_price = (bp + ap) / 2.0;

                                            // Send Tick
                                            let _ = tx
                                                .send(MarketEvent::Tick(Tick {
                                                    symbol: sym.to_string(),
                                                    price: mid_price,
                                                    volume: bs + as_size,
                                                    timestamp: Utc::now(),
                                                }))
                                                .await;

                                            // Send OrderBook
                                            let _ = tx
                                                .send(MarketEvent::OrderBook(OrderBookSnapshot {
                                                    symbol: sym.to_string(),
                                                    bids: vec![OrderBookLevel {
                                                        price: bp,
                                                        qty: bs,
                                                    }],
                                                    asks: vec![OrderBookLevel {
                                                        price: ap,
                                                        qty: as_size,
                                                    }],
                                                }))
                                                .await;
                                        }
                                    } else if msg_type == "error" {
                                        eprintln!("Alpaca WS Error: {:?}", item);
                                    }
                                }
                            }
                        }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to Alpaca WS: {}", e);
        }
    }
}
