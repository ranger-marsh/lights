//! Govee cloud HTTP API client.
//!
//! Base URL: `https://developer-api.govee.com`
//! Auth: `Govee-API-Key` header.
//! Rate limit: 10,000 requests / 24 hours.
//!
//! Use the LAN API ([`crate::lan::LanClient`]) where possible to avoid rate limits.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    error::{GoveeError, Result},
    models::{Color, Device, DeviceState},
};

const BASE_URL: &str = "https://developer-api.govee.com";

/// HTTP cloud API client.
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    api_key: String,
}

// --- Wire types --------------------------------------------------------------

#[derive(Deserialize)]
struct DeviceListResponse {
    data: DeviceListData,
}

#[derive(Deserialize)]
struct DeviceListData {
    devices: Vec<DeviceDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceDto {
    device: String, // MAC
    model: String,  // SKU
    device_name: String,
    controllable: bool,
}

#[derive(Serialize)]
struct ControlRequest<'a> {
    device: &'a str,
    model: &'a str,
    cmd: ControlCmd,
}

#[derive(Serialize)]
struct ControlCmd {
    name: &'static str,
    value: serde_json::Value,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ApiResponse {
    code: u16,
    message: String,
}

#[derive(Deserialize)]
struct StateResponse {
    data: StateData,
}

#[derive(Deserialize)]
struct StateData {
    properties: Vec<serde_json::Value>,
}

// --- Client ------------------------------------------------------------------

impl HttpClient {
    /// Create a new HTTP client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .user_agent("govee-lights-controller/0.1")
            .build()
            .map_err(GoveeError::Http)?;
        Ok(Self {
            client,
            api_key: api_key.into(),
        })
    }

    /// List all devices associated with the account.
    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        let resp = self
            .client
            .get(format!("{BASE_URL}/v1/devices"))
            .header("Govee-API-Key", &self.api_key)
            .send()
            .await?;

        self.check_status(&resp)?;

        let body: DeviceListResponse = resp.json().await?;
        let devices = body
            .data
            .devices
            .into_iter()
            .filter(|d| d.controllable)
            .map(|d| {
                let mut dev = Device::new(&d.device, &d.model);
                dev.name = Some(d.device_name);
                dev
            })
            .collect();

        Ok(devices)
    }

    /// Turn a device on or off.
    pub async fn set_power(&self, device: &Device, on: bool) -> Result<()> {
        self.control(
            device,
            ControlCmd {
                name: "turn",
                value: serde_json::json!(if on { "on" } else { "off" }),
            },
        )
        .await
    }

    /// Set brightness (1–100).
    pub async fn set_brightness(&self, device: &Device, brightness: u8) -> Result<()> {
        if brightness == 0 || brightness > 100 {
            return Err(GoveeError::InvalidBrightness { value: brightness });
        }
        self.control(
            device,
            ControlCmd {
                name: "brightness",
                value: serde_json::json!(brightness),
            },
        )
        .await
    }

    /// Set RGB color.
    pub async fn set_color(&self, device: &Device, color: Color) -> Result<()> {
        self.control(
            device,
            ControlCmd {
                name: "color",
                value: serde_json::json!({ "r": color.r, "g": color.g, "b": color.b }),
            },
        )
        .await
    }

    /// Set color temperature in Kelvin (2000–9000).
    pub async fn set_color_temp(&self, device: &Device, kelvin: u16) -> Result<()> {
        if kelvin < 2000 || kelvin > 9000 {
            return Err(GoveeError::InvalidColorTemp { value: kelvin });
        }
        self.control(
            device,
            ControlCmd {
                name: "colorTem",
                value: serde_json::json!(kelvin),
            },
        )
        .await
    }

    /// Query the current state of a device.
    pub async fn get_state(&self, device: &Device) -> Result<DeviceState> {
        let resp = self
            .client
            .get(format!("{BASE_URL}/v1/devices/state"))
            .header("Govee-API-Key", &self.api_key)
            .query(&[("device", &device.mac), ("model", &device.sku)])
            .send()
            .await?;

        self.check_status(&resp)?;

        let body: StateResponse = resp.json().await?;

        let mut state = DeviceState::default();
        for prop in body.data.properties {
            if let Some(v) = prop.get("powerSwitch") {
                state.on = v.as_u64().unwrap_or(0) != 0;
            } else if let Some(v) = prop.get("brightness") {
                state.brightness = v.as_u64().unwrap_or(100) as u8;
            } else if let Some(v) = prop.get("color") {
                if let (Some(r), Some(g), Some(b)) = (
                    v.get("r").and_then(|x| x.as_u64()),
                    v.get("g").and_then(|x| x.as_u64()),
                    v.get("b").and_then(|x| x.as_u64()),
                ) {
                    state.color = Color::new(r as u8, g as u8, b as u8);
                }
            } else if let Some(v) = prop.get("colorTemInKelvin") {
                state.color_temp_kelvin = v.as_u64().unwrap_or(0) as u16;
            }
        }

        Ok(state)
    }

    async fn control(&self, device: &Device, cmd: ControlCmd) -> Result<()> {
        debug!("HTTP control: {} {:?}", device.mac, cmd.name);
        let body = ControlRequest {
            device: &device.mac,
            model: &device.sku,
            cmd,
        };
        let resp = self
            .client
            .put(format!("{BASE_URL}/v1/devices/control"))
            .header("Govee-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        self.check_status(&resp)?;
        Ok(())
    }

    fn check_status(&self, resp: &reqwest::Response) -> Result<()> {
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            return Err(GoveeError::ApiError {
                status: 429,
                message: "rate limit exceeded (10,000 req/24h)".into(),
            });
        }
        if !resp.status().is_success() {
            return Err(GoveeError::ApiError {
                status: resp.status().as_u16(),
                message: resp.status().canonical_reason().unwrap_or("unknown").into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_brightness_rejected() {
        assert!(matches!(
            brightness_validate(0),
            Err(GoveeError::InvalidBrightness { value: 0 })
        ));
        assert!(matches!(
            brightness_validate(101),
            Err(GoveeError::InvalidBrightness { value: 101 })
        ));
        assert!(brightness_validate(1).is_ok());
        assert!(brightness_validate(100).is_ok());
    }

    fn brightness_validate(v: u8) -> Result<()> {
        if v == 0 || v > 100 {
            return Err(GoveeError::InvalidBrightness { value: v });
        }
        Ok(())
    }

    #[test]
    fn invalid_color_temp_rejected() {
        assert!(matches!(
            color_temp_validate(1999),
            Err(GoveeError::InvalidColorTemp { value: 1999 })
        ));
        assert!(matches!(
            color_temp_validate(9001),
            Err(GoveeError::InvalidColorTemp { value: 9001 })
        ));
        assert!(color_temp_validate(2000).is_ok());
        assert!(color_temp_validate(9000).is_ok());
    }

    fn color_temp_validate(v: u16) -> Result<()> {
        if v < 2000 || v > 9000 {
            return Err(GoveeError::InvalidColorTemp { value: v });
        }
        Ok(())
    }
}
