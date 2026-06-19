use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use chrono::Utc;
use hft_core::{MarketEvent, Tick, OrderBookSnapshot, OrderBookLevel};

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct BinanceBookTicker {
    s: String, // symbol
    b: String, // best bid price
    B: String, // best bid qty
    a: String, // best ask price
    A: String, // best ask qty
}

pub async fn start_binance_ws(symbols: Vec<String>, tx: mpsc::Sender<MarketEvent>) {
    let mut streams = Vec::new();
    for sym in symbols {
        streams.push(format!("{}@bookTicker", sym.to_lowercase()));
    }
    
    let stream_names = streams.join("/");
    let url = format!("wss://stream.testnet.binance.vision/ws/{}", stream_names);
    
    println!("Connecting to Binance Testnet WebSocket: {}", url);
    
    match connect_async(&url).await {
        Ok((mut ws_stream, _)) => {
            println!("Connected to Binance WebSocket!");
            while let Some(msg) = ws_stream.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(data) = serde_json::from_str::<BinanceBookTicker>(&text) {
                        let bid_price: f64 = data.b.parse().unwrap_or(0.0);
                        let ask_price: f64 = data.a.parse().unwrap_or(0.0);
                        let bid_qty: f64 = data.B.parse().unwrap_or(0.0);
                        let ask_qty: f64 = data.A.parse().unwrap_or(0.0);
                        
                        let mid_price = (bid_price + ask_price) / 2.0;
                        
                        // Send Tick
                        let _ = tx.send(MarketEvent::Tick(Tick {
                            symbol: data.s.clone(),
                            price: mid_price,
                            volume: bid_qty + ask_qty,
                            timestamp: Utc::now(),
                        })).await;
                        
                        // Send Top-of-book
                        let _ = tx.send(MarketEvent::OrderBook(OrderBookSnapshot {
                            symbol: data.s.clone(),
                            bids: vec![OrderBookLevel { price: bid_price, qty: bid_qty }],
                            asks: vec![OrderBookLevel { price: ask_price, qty: ask_qty }],
                        })).await;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to Binance WS: {}", e);
        }
    }
}
