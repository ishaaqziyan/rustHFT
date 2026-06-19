use reqwest::Client;
use serde_json::json;

pub struct AlpacaClient {
    client: Client,
    api_key: String,
    api_secret: String,
    base_url: String,
}

impl AlpacaClient {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_secret,
            base_url: "https://paper-api.alpaca.markets/v2".to_string(), // Always use paper API
        }
    }

    pub async fn place_order(&self, symbol: &str, qty: f64, side: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/orders", self.base_url);
        
        let payload = json!({
            "symbol": symbol,
            "qty": qty.to_string(),
            "side": side,
            "type": "market",
            "time_in_force": "day"
        });

        let res = self.client.post(&url)
            .header("APCA-API-KEY-ID", &self.api_key)
            .header("APCA-API-SECRET-KEY", &self.api_secret)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            eprintln!("Alpaca Order Error: {}", error_text);
            return Err(format!("Failed to place order: {}", error_text).into());
        }

        println!("Successfully submitted {} order for {} to Alpaca Paper API", side, symbol);
        Ok(())
    }
}
