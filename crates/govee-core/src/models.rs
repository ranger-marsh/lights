use serde::{Deserialize, Serialize};

/// RGB color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn red() -> Self { Self::new(255, 0, 0) }
    pub fn green() -> Self { Self::new(0, 255, 0) }
    pub fn blue() -> Self { Self::new(0, 0, 255) }
    pub fn white() -> Self { Self::new(255, 255, 255) }
    pub fn off() -> Self { Self::new(0, 0, 0) }

    /// Convert to HSV. Returns (hue 0–360, saturation 0–1, value 0–1).
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta < f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let h = if h < 0.0 { h + 360.0 } else { h };
        let s = if max < f32::EPSILON { 0.0 } else { delta / max };

        (h, s, max)
    }

    /// Create from HSV. Hue 0–360, saturation and value 0–1.
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match h as u32 {
            0..=59   => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _         => (c, 0.0, x),
        };

        Self::new(
            ((r + m) * 255.0).round() as u8,
            ((g + m) * 255.0).round() as u8,
            ((b + m) * 255.0).round() as u8,
        )
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// A Govee light device discovered on the local network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// Device MAC address (used as unique identifier).
    pub mac: String,
    /// Device model / SKU (e.g. "H6072").
    pub sku: String,
    /// Human-readable name (from device or config).
    pub name: Option<String>,
    /// IP address on the local network (LAN API only).
    pub ip: Option<std::net::IpAddr>,
}

impl Device {
    pub fn new(mac: impl Into<String>, sku: impl Into<String>) -> Self {
        Self {
            mac: mac.into(),
            sku: sku.into(),
            name: None,
            ip: None,
        }
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.sku)
    }
}

/// Current state of a Govee device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceState {
    /// Whether the light is on.
    pub on: bool,
    /// Brightness level 1–100.
    pub brightness: u8,
    /// Current color.
    pub color: Color,
    /// Color temperature in Kelvin (2000–9000), or 0 if using RGB mode.
    pub color_temp_kelvin: u16,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            on: false,
            brightness: 100,
            color: Color::white(),
            color_temp_kelvin: 0,
        }
    }
}

/// Commands that can be sent to a Govee device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    TurnOn,
    TurnOff,
    SetBrightness(u8),
    SetColor(Color),
    SetColorTemp(u16),
    QueryState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_display() {
        assert_eq!(Color::red().to_string(), "#FF0000");
        assert_eq!(Color::new(0, 128, 255).to_string(), "#0080FF");
    }

    #[test]
    fn color_round_trip_hsv() {
        let original = Color::new(255, 128, 64);
        let (h, s, v) = original.to_hsv();
        let restored = Color::from_hsv(h, s, v);
        // Allow ±1 rounding error per channel
        assert!((original.r as i16 - restored.r as i16).abs() <= 1);
        assert!((original.g as i16 - restored.g as i16).abs() <= 1);
        assert!((original.b as i16 - restored.b as i16).abs() <= 1);
    }

    #[test]
    fn color_white_hsv() {
        let (_, s, v) = Color::white().to_hsv();
        assert!(s < f32::EPSILON);
        assert!((v - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn device_display_name_falls_back_to_sku() {
        let d = Device::new("AA:BB:CC:DD", "H6072");
        assert_eq!(d.display_name(), "H6072");
    }

    #[test]
    fn device_display_name_uses_name_when_set() {
        let mut d = Device::new("AA:BB:CC:DD", "H6072");
        d.name = Some("Living Room".to_string());
        assert_eq!(d.display_name(), "Living Room");
    }

    #[test]
    fn device_state_default_is_off() {
        let state = DeviceState::default();
        assert!(!state.on);
        assert_eq!(state.brightness, 100);
    }
}
