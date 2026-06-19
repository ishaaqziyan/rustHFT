use chrono::Utc;
use hft_core::Tick;
use reqwest::Client;
use serde_json::Value;

pub struct YahooClient {
    client: Client,
}

impl YahooClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_latest_quotes(
        &self,
        symbols: &[&str],
    ) -> Result<Vec<Tick>, Box<dyn std::error::Error + Send + Sync>> {
        let sym_str = symbols.join(",");
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}",
            sym_str
        );

        let res = self.client.get(&url).send().await?;

        let data: Value = res.json().await?;
        let mut ticks = Vec::new();

        if let Some(result) = data
            .get("quoteResponse")
            .and_then(|qr| qr.get("result"))
            .and_then(|r| r.as_array())
        {
            for quote in result {
                if let (Some(symbol), Some(price)) = (
                    quote.get("symbol").and_then(|v| v.as_str()),
                    quote.get("regularMarketPrice").and_then(|v| v.as_f64()),
                ) {
                    ticks.push(Tick {
                        symbol: symbol.to_string(),
                        price,
                        volume: quote
                            .get("regularMarketVolume")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(100.0),
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        Ok(ticks)
    }
}
