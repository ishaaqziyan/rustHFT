# 🚀 Universal StatArb Engine (Rust)

A high-frequency statistical arbitrage paper-trading engine written in blazing-fast Rust. It is designed to trade both Cryptocurrency and traditional US/International Stocks concurrently, utilizing a hybrid real-time data architecture and decoupled execution layers.

![Dashboard Preview](https://img.shields.io/badge/UI-Dioxus-blue)
![Language](https://img.shields.io/badge/Language-Rust-orange)
![Execution](https://img.shields.io/badge/Execution-Paper_Trading-success)
![CI](https://github.com/YOUR_GITHUB_USERNAME/rust-hft-system/actions/workflows/ci.yml/badge.svg)

## ✨ Features

- **Multi-Asset Architecture**: Trade Crypto (BTC, ETH, SOL) and Equities (US & Swiss Stocks) within the exact same strategy pipeline.
- **Hybrid Real-Time Data**:
  - **Crypto**: Lightning-fast streaming tick data via **Binance WebSockets** (`wss://stream.testnet.binance.vision/ws/`).
  - **Equities**: Sub-millisecond US stock market data via **Alpaca WebSockets** (`wss://stream.data.alpaca.markets/v2/iex`).
- **Statistical Arbitrage Model**: Uses Ordinary Least Squares (OLS) co-integration with dynamic Z-score calculations to find mean-reverting spreads between highly correlated assets.
- **Decoupled Execution**: 
  - **Crypto**: Orders are cryptographically signed (HMAC-SHA256) and routed to the **Binance Spot Testnet API** (`testnet.binance.vision`).
  - **Equities**: Orders are pushed to **Alpaca's REST API** (`paper-api.alpaca.markets`) for real execution tracking while maintaining localized sub-millisecond PnL simulation.
- **Doppler Secrets Integration**: All API keys are securely managed and injected via Doppler, completely removing plaintext secrets from the UI and source code.
- **Native GUI**: A sleek, reactive dashboard built with Dioxus, delivering a rich interface with order book visualization, live execution logs, and PnL tracking directly from the Rust binary.
- **Continuous Integration (CI)**: Automated GitHub Actions workflows configured in `.github/workflows/ci.yml` that run format checks, linting (clippy), build steps, and `cargo-nextest` unit tests across the entire workspace on every commit and PR.

---

## 🛠️ System Architecture

1. **`dashboard` (Dioxus)**: The reactive frontend GUI. Receives high-speed async updates via `tokio::mpsc` channels and visualizes the limit order book, executions, and the strategy's dynamic Beta (β) and Z-score models.
2. **`engine` (Tokio)**: The orchestrator. It manages the background data streaming tasks, dynamically tearing down and re-establishing WebSocket connections when the user changes trading pairs. It routes ticks into the strategy and routes execution commands to the brokers.
3. **`exchange` (WebSockets & REST)**: The broker clients.
   - `binance_ws.rs`: Streams real-time top-of-book order book data for Crypto pairs.
   - `binance.rs`: Executes signed market orders on the Binance Spot Testnet API.
   - `alpaca_ws.rs`: Connects, authenticates, and streams real-time IEX quote data for US Equities.
   - `alpaca.rs`: Executes market orders on the Alpaca V2 Paper Trading API.
4. **`strategy`**: The quantitative math layer. Tracks tick-by-tick prices, maintains position sizing, computes moving averages, and emits `SubmitOrder` commands when the Z-score crosses threshold boundaries.

---

## 🚀 Getting Started

### Prerequisites

1. **Rust**: Install the latest stable toolchain via [rustup](https://rustup.rs/).
2. **Just**: Install the Just command runner (`cargo install just`).
3. **Doppler CLI**: Install Doppler for secrets management.

### Configuration

You must provide API keys to trade US Equities and Crypto. These are securely managed via Doppler.

1. Create a Doppler project for your HFT system.
2. Add the following secrets to your Doppler environment:
   - `APCA_API_KEY_ID="your_alpaca_paper_key"`
   - `APCA_API_SECRET_KEY="your_alpaca_paper_secret"`
   - `BINANCE_API_KEY="your_binance_testnet_key"`
   - `BINANCE_API_SECRET="your_binance_testnet_secret"`
3. Link your local directory to Doppler:
   ```bash
   doppler setup
   ```

### Running the Engine

Boot up the engine and launch the Dioxus UI securely:

```bash
just run-ui
```

*This command automatically utilizes `doppler run` to inject your credentials directly into the running process.*

---

## 📈 Trading Logic (StatArb)

The engine monitors a pair of assets (e.g., `AAPL` and `MSFT`). 
1. It calculates the **Spread** based on a dynamically adjusting **Beta (β)**.
2. It standardizes the spread into a **Z-Score**.
3. **Entry**: If the Z-Score deviates beyond `+2.0` or `-2.0`, it assumes the pair has diverged from historical correlation and enters a long/short position to capture the mean reversion.
4. **Exit**: When the Z-Score reverts back to `0.0`, the positions are flattened to lock in the profit.

---

## ⚠️ Disclaimer

This is a **Paper Trading Simulation Engine**. The OLS StatArb strategy implemented is highly simplified for demonstrative purposes and does not account for slippage, latency arbitrage, structural market regime shifts, or exchange fees. **Do not plug this into a live production account without extensive quantitative backtesting.**
