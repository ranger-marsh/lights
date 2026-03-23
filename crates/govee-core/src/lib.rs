//! Govee lights control library.
//!
//! Designed for **air-gapped / local-network** operation.
//! The LAN UDP API is the primary (and default) control path — no internet required.
//!
//! The cloud HTTP API is gated behind the `http-api` cargo feature and is
//! disabled by default. Enable it only when internet access is available.
//!
//! # Example
//!
//! ```no_run
//! use govee_core::{lan::LanClient, models::Color};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = LanClient::new().await?;
//!     let devices = client.discover(std::time::Duration::from_secs(2)).await?;
//!
//!     if let Some(device) = devices.first() {
//!         client.set_color(device, Color::new(255, 0, 128)).await?;
//!         client.set_brightness(device, 80).await?;
//!     }
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod lan;
pub mod models;

/// Cloud HTTP API — requires `http-api` feature and internet access.
#[cfg(feature = "http-api")]
pub mod http;
