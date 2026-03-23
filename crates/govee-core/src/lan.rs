//! Govee LAN API client (local UDP control).
//!
//! Requires "LAN Control" to be enabled per-device in the Govee Home app.
//! Communicates over UDP — no internet required, no rate limits.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, time::timeout};
use tracing::{debug, warn};

use crate::{
    error::{GoveeError, Result},
    models::{Color, Command, Device, DeviceState},
};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const DISCOVERY_PORT: u16 = 4001;
const LISTEN_PORT: u16 = 4002;
const CONTROL_PORT: u16 = 4003;

// --- Wire format types -------------------------------------------------------

#[derive(Serialize)]
struct LanMessage<T: Serialize> {
    msg: LanMessageInner<T>,
}

#[derive(Serialize)]
struct LanMessageInner<T: Serialize> {
    cmd: &'static str,
    data: T,
}

impl<T: Serialize> LanMessage<T> {
    fn new(cmd: &'static str, data: T) -> Self {
        Self {
            msg: LanMessageInner { cmd, data },
        }
    }
}

#[derive(Serialize)]
struct ScanData {
    account_topic: &'static str,
}

#[derive(Deserialize, Debug)]
struct ScanResponse {
    msg: ScanResponseInner,
}

#[derive(Deserialize, Debug)]
struct ScanResponseInner {
    data: ScanResponseData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ScanResponseData {
    ip: String,
    device: String, // MAC address
    sku: String,
    #[serde(default)]
    device_name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StatusResponse {
    msg: StatusResponseInner,
}

#[derive(Deserialize, Debug)]
struct StatusResponseInner {
    data: StatusResponseData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusResponseData {
    on_off: u8,
    brightness: u8,
    color: ColorData,
    color_temperature_kelvin: u16,
}

#[derive(Deserialize, Debug)]
struct ColorData {
    r: u8,
    g: u8,
    b: u8,
}

// --- Control data types -------------------------------------------------------

#[derive(Serialize)]
struct TurnData {
    value: u8,
}

#[derive(Serialize)]
struct BrightnessData {
    value: u8,
}

#[derive(Serialize)]
struct ColorwcData {
    color: ColorPayload,
    #[serde(rename = "colorTemInKelvin")]
    color_temp_in_kelvin: u16,
}

#[derive(Serialize)]
struct ColorPayload {
    r: u8,
    g: u8,
    b: u8,
}

// --- Client ------------------------------------------------------------------

/// LAN API client. Discovers and controls Govee devices on the local network.
pub struct LanClient {
    socket: UdpSocket,
}

impl LanClient {
    /// Create a new LAN client, binding to the listen port.
    pub async fn new() -> Result<Self> {
        let socket = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            LISTEN_PORT,
        ))
        .await?;
        socket.set_broadcast(true)?;
        socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)?;
        Ok(Self { socket })
    }

    /// Discover all Govee devices on the local network within `timeout_duration`.
    pub async fn discover(&self, timeout_duration: Duration) -> Result<Vec<Device>> {
        let scan_msg = LanMessage::new("scan", ScanData { account_topic: "reserve" });
        let payload = serde_json::to_vec(&scan_msg)?;

        self.socket
            .send_to(&payload, (MULTICAST_ADDR, DISCOVERY_PORT))
            .await?;

        let mut devices = Vec::new();
        let mut buf = vec![0u8; 4096];

        let deadline = tokio::time::Instant::now() + timeout_duration;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match timeout(remaining, self.socket.recv_from(&mut buf)).await {
                Ok(Ok((n, addr))) => {
                    debug!("received {} bytes from {}", n, addr);
                    match serde_json::from_slice::<ScanResponse>(&buf[..n]) {
                        Ok(resp) => {
                            let d = resp.msg.data;
                            let ip: IpAddr = d.ip.parse().unwrap_or(addr.ip());
                            let mut device = Device::new(&d.device, &d.sku);
                            device.ip = Some(ip);
                            device.name = d.device_name;
                            devices.push(device);
                        }
                        Err(e) => warn!("failed to parse scan response: {e}"),
                    }
                }
                Ok(Err(e)) => return Err(GoveeError::Network(e)),
                Err(_) => break, // timeout
            }
        }

        Ok(devices)
    }

    /// Send a command to a device via the LAN API.
    pub async fn send_command(&self, device: &Device, command: Command) -> Result<()> {
        let ip = device.ip.ok_or_else(|| {
            GoveeError::DeviceNotFound(format!("device {} has no IP address", device.mac))
        })?;

        let payload = self.encode_command(command)?;
        self.socket
            .send_to(&payload, SocketAddr::new(ip, CONTROL_PORT))
            .await?;
        Ok(())
    }

    /// Query the current state of a device.
    pub async fn get_state(
        &self,
        device: &Device,
        response_timeout: Duration,
    ) -> Result<DeviceState> {
        self.send_command(device, Command::QueryState).await?;

        let mut buf = vec![0u8; 4096];
        match timeout(response_timeout, self.socket.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) => {
                let resp: StatusResponse = serde_json::from_slice(&buf[..n])?;
                let d = resp.msg.data;
                Ok(DeviceState {
                    on: d.on_off != 0,
                    brightness: d.brightness,
                    color: Color::new(d.color.r, d.color.g, d.color.b),
                    color_temp_kelvin: d.color_temperature_kelvin,
                })
            }
            Ok(Err(e)) => Err(GoveeError::Network(e)),
            Err(_) => Err(GoveeError::Timeout),
        }
    }

    /// Convenience: turn a device on or off.
    pub async fn set_power(&self, device: &Device, on: bool) -> Result<()> {
        self.send_command(
            device,
            if on { Command::TurnOn } else { Command::TurnOff },
        )
        .await
    }

    /// Convenience: set brightness (1–100).
    pub async fn set_brightness(&self, device: &Device, brightness: u8) -> Result<()> {
        if brightness == 0 || brightness > 100 {
            return Err(GoveeError::InvalidBrightness { value: brightness });
        }
        self.send_command(device, Command::SetBrightness(brightness)).await
    }

    /// Convenience: set RGB color.
    pub async fn set_color(&self, device: &Device, color: Color) -> Result<()> {
        self.send_command(device, Command::SetColor(color)).await
    }

    /// Convenience: set color temperature in Kelvin (2000–9000).
    pub async fn set_color_temp(&self, device: &Device, kelvin: u16) -> Result<()> {
        if kelvin < 2000 || kelvin > 9000 {
            return Err(GoveeError::InvalidColorTemp { value: kelvin });
        }
        self.send_command(device, Command::SetColorTemp(kelvin)).await
    }

    fn encode_command(&self, command: Command) -> Result<Vec<u8>> {
        let payload = match command {
            Command::TurnOn => {
                serde_json::to_vec(&LanMessage::new("turn", TurnData { value: 1 }))?
            }
            Command::TurnOff => {
                serde_json::to_vec(&LanMessage::new("turn", TurnData { value: 0 }))?
            }
            Command::SetBrightness(v) => {
                serde_json::to_vec(&LanMessage::new("brightness", BrightnessData { value: v }))?
            }
            Command::SetColor(c) => serde_json::to_vec(&LanMessage::new(
                "colorwc",
                ColorwcData {
                    color: ColorPayload {
                        r: c.r,
                        g: c.g,
                        b: c.b,
                    },
                    color_temp_in_kelvin: 0,
                },
            ))?,
            Command::SetColorTemp(k) => serde_json::to_vec(&LanMessage::new(
                "colorwc",
                ColorwcData {
                    color: ColorPayload { r: 0, g: 0, b: 0 },
                    color_temp_in_kelvin: k,
                },
            ))?,
            Command::QueryState => {
                serde_json::to_vec(&LanMessage::new("devStatus", serde_json::json!({})))?
            }
        };
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn decode(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).unwrap()
    }

    fn encode(command: Command) -> Vec<u8> {
        match command {
            Command::TurnOn => {
                serde_json::to_vec(&LanMessage::new("turn", TurnData { value: 1 })).unwrap()
            }
            Command::TurnOff => {
                serde_json::to_vec(&LanMessage::new("turn", TurnData { value: 0 })).unwrap()
            }
            Command::SetBrightness(v) => {
                serde_json::to_vec(&LanMessage::new("brightness", BrightnessData { value: v }))
                    .unwrap()
            }
            Command::SetColor(c) => serde_json::to_vec(&LanMessage::new(
                "colorwc",
                ColorwcData {
                    color: ColorPayload {
                        r: c.r,
                        g: c.g,
                        b: c.b,
                    },
                    color_temp_in_kelvin: 0,
                },
            ))
            .unwrap(),
            Command::SetColorTemp(k) => serde_json::to_vec(&LanMessage::new(
                "colorwc",
                ColorwcData {
                    color: ColorPayload { r: 0, g: 0, b: 0 },
                    color_temp_in_kelvin: k,
                },
            ))
            .unwrap(),
            Command::QueryState => {
                serde_json::to_vec(&LanMessage::new("devStatus", serde_json::json!({})))
                    .unwrap()
            }
        }
    }

    #[test]
    fn encode_turn_on() {
        let v = decode(&encode(Command::TurnOn));
        assert_eq!(v["msg"]["cmd"], "turn");
        assert_eq!(v["msg"]["data"]["value"], 1);
    }

    #[test]
    fn encode_turn_off() {
        let v = decode(&encode(Command::TurnOff));
        assert_eq!(v["msg"]["data"]["value"], 0);
    }

    #[test]
    fn encode_brightness() {
        let v = decode(&encode(Command::SetBrightness(75)));
        assert_eq!(v["msg"]["cmd"], "brightness");
        assert_eq!(v["msg"]["data"]["value"], 75);
    }

    #[test]
    fn encode_color() {
        let v = decode(&encode(Command::SetColor(Color::new(255, 128, 0))));
        assert_eq!(v["msg"]["cmd"], "colorwc");
        assert_eq!(v["msg"]["data"]["color"]["r"], 255);
        assert_eq!(v["msg"]["data"]["color"]["g"], 128);
        assert_eq!(v["msg"]["data"]["color"]["b"], 0);
        assert_eq!(v["msg"]["data"]["colorTemInKelvin"], 0);
    }

    #[test]
    fn encode_color_temp() {
        let v = decode(&encode(Command::SetColorTemp(4000)));
        assert_eq!(v["msg"]["cmd"], "colorwc");
        assert_eq!(v["msg"]["data"]["colorTemInKelvin"], 4000);
    }

    #[test]
    fn encode_query_state() {
        let v = decode(&encode(Command::QueryState));
        assert_eq!(v["msg"]["cmd"], "devStatus");
    }
}
