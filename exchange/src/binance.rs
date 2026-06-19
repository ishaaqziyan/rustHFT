use reqwest::Client;
use serde_json::Value;
use chrono::Utc;
use hft_core::Tick;
use hmac::{Hmac, Mac, KeyInit};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct BinanceClient {
    client: Client,
    api_key: String,
    api_secret: String,
}

impl BinanceClient {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_secret,
        }
    }

    pub async fn fetch_latest_quotes(&self, symbols: &[&str]) -> Result<Vec<Tick>, Box<dyn std::error::Error + Send + Sync>> {
        // Binance accepts symbols in JSON array format like ["BTCUSDT","ETHUSDT"]
        let symbols_json = serde_json::to_string(symbols)?;
        let url = format!("https://api.binance.com/api/v3/ticker/bookTicker?symbols={}", symbols_json);

        let res = self.client.get(&url).send().await?;
        let data: Value = res.json().await?;
        let mut ticks = Vec::new();

        if let Some(arr) = data.as_array() {
            for item in arr {
                if let (Some(symbol), Some(bid_price_str), Some(ask_price_str), Some(bid_qty_str)) = (
                    item.get("symbol").and_then(|v| v.as_str()),
                    item.get("bidPrice").and_then(|v| v.as_str()),
                    item.get("askPrice").and_then(|v| v.as_str()),
                    item.get("bidQty").and_then(|v| v.as_str())
                ) {
                    if let (Ok(bid), Ok(ask), Ok(qty)) = (bid_price_str.parse::<f64>(), ask_price_str.parse::<f64>(), bid_qty_str.parse::<f64>()) {
                        ticks.push(Tick {
                            symbol: symbol.to_string(),
                            price: (bid + ask) / 2.0,
                            volume: qty,
                            timestamp: Utc::now(),
                        });
                    }
                }
            }
        }
        Ok(ticks)
    }

    pub async fn place_order(&self, symbol: &str, qty: f64, side: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = Utc::now().timestamp_millis();
        let side_str = side.to_uppercase(); // "BUY" or "SELL"
        
        let query = format!("symbol={}&side={}&type=MARKET&quantity={:.5}&recvWindow=60000&timestamp={}", symbol, side_str, qty, timestamp);
        
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(query.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        
        let final_query = format!("{}&signature={}", query, signature);
        let url = format!("https://testnet.binance.vision/api/v3/order?{}", final_query);

        let res = self.client.post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            eprintln!("Binance Order Error: {}", error_text);
            return Err(format!("Failed to place Binance order: {}", error_text).into());
        }

        println!("Successfully submitted {} order for {} to Binance Testnet API", side_str, symbol);
        Ok(())
    }
}
