//! Built-in scene presets — static palette data, no persistence required.
//!
//! Each [`Scene`] carries a brightness level (1–100) and a colour palette.
//! When applied to N devices, device `i` receives `palette[i % palette.len()]`.
//!
//! # Categories
//! 1. **Nature**  — forest, ocean, weather, and flora palettes
//! 2. **Holiday** — seasonal and cultural celebrations
//! 3. **Relax**   — low-stimulation, wellness-oriented moods
//! 4. **Cosmic**  — space and astronomy themes
//! 5. **Party**   — high-energy, vivid entertainment palettes

// ── Data types ────────────────────────────────────────────────────────────────

/// A single built-in scene preset.
pub struct Scene {
    /// Short human-readable name shown on the button.
    pub name: &'static str,
    /// Default brightness (1–100) applied to every device in the scene.
    pub brightness: u8,
    /// Colour palette as `(r, g, b)` tuples.  Distributed across devices
    /// in round-robin order.
    pub palette: &'static [(u8, u8, u8)],
}

/// A named group of related [`Scene`]s.
pub struct Category {
    pub name: &'static str,
    pub scenes: &'static [Scene],
}

// ── Category registry ─────────────────────────────────────────────────────────

/// All built-in scene categories in display order.
pub static CATEGORIES: &[Category] = &[
    Category { name: "Nature",  scenes: NATURE  },
    Category { name: "Holiday", scenes: HOLIDAY },
    Category { name: "Relax",   scenes: RELAX   },
    Category { name: "Cosmic",  scenes: COSMIC  },
    Category { name: "Party",   scenes: PARTY   },
];

// ── Nature ────────────────────────────────────────────────────────────────────

pub static NATURE: &[Scene] = &[
    Scene {
        name: "Forest",
        brightness: 80,
        palette: &[(34, 139, 34), (0, 100, 0), (144, 238, 144), (0, 128, 0)],
    },
    Scene {
        name: "Ocean",
        brightness: 85,
        palette: &[(0, 119, 190), (0, 206, 209), (64, 164, 223), (0, 150, 200)],
    },
    Scene {
        name: "Sunset",
        brightness: 90,
        palette: &[(255, 94, 77), (255, 154, 0), (200, 80, 192), (255, 70, 30)],
    },
    Scene {
        name: "Sunrise",
        brightness: 75,
        palette: &[(255, 183, 197), (255, 218, 185), (255, 223, 0), (255, 160, 80)],
    },
    Scene {
        name: "Desert",
        brightness: 85,
        palette: &[(210, 180, 140), (205, 92, 92), (255, 140, 0), (194, 144, 80)],
    },
    Scene {
        name: "Aurora",
        brightness: 70,
        palette: &[(0, 255, 127), (75, 0, 130), (0, 191, 255), (100, 220, 180)],
    },
    Scene {
        name: "Firefly",
        brightness: 60,
        palette: &[(180, 255, 0), (255, 220, 0), (100, 200, 50), (200, 255, 80)],
    },
    Scene {
        name: "Coral Reef",
        brightness: 90,
        palette: &[(255, 127, 80), (0, 206, 209), (255, 165, 0), (255, 80, 120)],
    },
    Scene {
        name: "Thunderstorm",
        brightness: 55,
        palette: &[(25, 25, 112), (200, 200, 255), (65, 105, 225), (120, 120, 200)],
    },
    Scene {
        name: "Meadow",
        brightness: 85,
        palette: &[(124, 252, 0), (173, 255, 47), (240, 255, 240), (154, 205, 50)],
    },
    Scene {
        name: "Autumn",
        brightness: 80,
        palette: &[(255, 100, 10), (180, 30, 30), (218, 165, 32), (160, 60, 0)],
    },
    Scene {
        name: "Rainforest",
        brightness: 70,
        palette: &[(0, 80, 0), (50, 205, 50), (34, 139, 34), (0, 160, 60)],
    },
];

// ── Holiday ───────────────────────────────────────────────────────────────────

pub static HOLIDAY: &[Scene] = &[
    Scene {
        name: "Christmas",
        brightness: 90,
        palette: &[(220, 20, 60), (0, 128, 0), (220, 20, 60), (0, 160, 0)],
    },
    Scene {
        name: "Halloween",
        brightness: 85,
        palette: &[(255, 100, 0), (128, 0, 128), (255, 80, 0), (100, 0, 100)],
    },
    Scene {
        name: "Valentine's",
        brightness: 80,
        palette: &[(220, 20, 60), (255, 105, 180), (255, 20, 80), (255, 180, 200)],
    },
    Scene {
        name: "4th of July",
        brightness: 95,
        palette: &[(220, 20, 60), (240, 240, 255), (0, 0, 200), (220, 20, 60)],
    },
    Scene {
        name: "Easter",
        brightness: 80,
        palette: &[(255, 182, 193), (221, 160, 221), (152, 251, 152), (255, 255, 153)],
    },
    Scene {
        name: "St. Patrick's",
        brightness: 90,
        palette: &[(0, 128, 0), (0, 201, 87), (128, 255, 0), (0, 160, 60)],
    },
    Scene {
        name: "Hanukkah",
        brightness: 85,
        palette: &[(0, 70, 200), (192, 192, 192), (255, 255, 255), (30, 100, 220)],
    },
    Scene {
        name: "New Year's",
        brightness: 95,
        palette: &[(255, 215, 0), (192, 192, 192), (255, 240, 200), (200, 180, 0)],
    },
    Scene {
        name: "Thanksgiving",
        brightness: 80,
        palette: &[(255, 140, 0), (139, 69, 19), (218, 165, 32), (200, 100, 20)],
    },
    Scene {
        name: "Diwali",
        brightness: 95,
        palette: &[(255, 215, 0), (255, 140, 0), (200, 20, 20), (255, 180, 0)],
    },
    Scene {
        name: "Mardi Gras",
        brightness: 90,
        palette: &[(128, 0, 128), (255, 215, 0), (0, 128, 0), (180, 0, 180)],
    },
    Scene {
        name: "Winter",
        brightness: 70,
        palette: &[(20, 20, 100), (200, 225, 255), (150, 190, 255), (220, 235, 255)],
    },
];

// ── Relax ─────────────────────────────────────────────────────────────────────

pub static RELAX: &[Scene] = &[
    Scene {
        name: "Meditation",
        brightness: 50,
        palette: &[(100, 50, 200), (150, 100, 255), (80, 40, 180), (130, 80, 220)],
    },
    Scene {
        name: "Spa",
        brightness: 65,
        palette: &[(152, 251, 152), (240, 248, 255), (173, 216, 230), (180, 255, 200)],
    },
    Scene {
        name: "Sleep",
        brightness: 15,
        palette: &[(200, 10, 10), (180, 8, 8)],
    },
    Scene {
        name: "Reading",
        brightness: 70,
        palette: &[(255, 230, 180), (255, 220, 160)],
    },
    Scene {
        name: "Movie Night",
        brightness: 25,
        palette: &[(180, 100, 20), (160, 80, 10)],
    },
    Scene {
        name: "Yoga",
        brightness: 60,
        palette: &[(230, 190, 255), (255, 218, 185), (200, 180, 255), (255, 200, 200)],
    },
    Scene {
        name: "Bath Time",
        brightness: 65,
        palette: &[(100, 149, 237), (175, 238, 238), (120, 180, 255), (150, 220, 240)],
    },
    Scene {
        name: "Candlelight",
        brightness: 45,
        palette: &[(255, 147, 41), (220, 100, 20), (240, 130, 30)],
    },
    Scene {
        name: "Wind Down",
        brightness: 35,
        palette: &[(200, 80, 30), (180, 60, 20), (160, 50, 10)],
    },
    Scene {
        name: "Campfire",
        brightness: 70,
        palette: &[(255, 100, 20), (200, 30, 30), (255, 200, 0), (220, 80, 10)],
    },
    Scene {
        name: "Mindfulness",
        brightness: 55,
        palette: &[(138, 43, 226), (0, 139, 139), (120, 40, 200), (0, 120, 120)],
    },
    Scene {
        name: "Cloud Nine",
        brightness: 60,
        palette: &[(230, 240, 255), (200, 210, 255), (220, 220, 255), (210, 225, 255)],
    },
];

// ── Cosmic ────────────────────────────────────────────────────────────────────

pub static COSMIC: &[Scene] = &[
    Scene {
        name: "Aurora Borealis",
        brightness: 75,
        palette: &[(0, 255, 127), (75, 0, 130), (0, 191, 255), (100, 220, 180)],
    },
    Scene {
        name: "Galaxy",
        brightness: 65,
        palette: &[(75, 0, 130), (25, 25, 112), (255, 20, 147), (100, 0, 160)],
    },
    Scene {
        name: "Nebula",
        brightness: 70,
        palette: &[(255, 0, 200), (0, 200, 255), (148, 0, 211), (200, 0, 255)],
    },
    Scene {
        name: "Supernova",
        brightness: 100,
        palette: &[(255, 255, 200), (255, 200, 0), (255, 100, 0), (255, 240, 180)],
    },
    Scene {
        name: "Milky Way",
        brightness: 60,
        palette: &[(200, 220, 255), (220, 210, 255), (180, 200, 240), (210, 215, 250)],
    },
    Scene {
        name: "Solar Flare",
        brightness: 95,
        palette: &[(255, 140, 0), (255, 220, 0), (220, 40, 0), (255, 180, 20)],
    },
    Scene {
        name: "Black Hole",
        brightness: 12,
        palette: &[(30, 0, 60), (60, 0, 80), (20, 0, 40)],
    },
    Scene {
        name: "Comet",
        brightness: 80,
        palette: &[(230, 250, 255), (30, 100, 255), (180, 230, 255), (60, 140, 255)],
    },
    Scene {
        name: "Mars",
        brightness: 75,
        palette: &[(180, 50, 30), (220, 80, 20), (150, 30, 10), (200, 60, 20)],
    },
    Scene {
        name: "Moonrise",
        brightness: 55,
        palette: &[(180, 190, 210), (210, 220, 240), (240, 245, 255), (195, 205, 225)],
    },
    Scene {
        name: "Pulsar",
        brightness: 85,
        palette: &[(0, 50, 255), (200, 200, 255), (30, 80, 255), (180, 190, 255)],
    },
    Scene {
        name: "Wormhole",
        brightness: 70,
        palette: &[(0, 80, 100), (100, 0, 150), (0, 100, 200), (80, 20, 140)],
    },
];

// ── Party ─────────────────────────────────────────────────────────────────────

pub static PARTY: &[Scene] = &[
    Scene {
        name: "Disco",
        brightness: 100,
        palette: &[(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (255, 0, 255)],
    },
    Scene {
        name: "Tropical",
        brightness: 95,
        palette: &[(0, 210, 210), (255, 20, 147), (255, 230, 0), (0, 200, 180)],
    },
    Scene {
        name: "Rave",
        brightness: 100,
        palette: &[(0, 100, 255), (0, 255, 100), (255, 0, 200), (150, 0, 255)],
    },
    Scene {
        name: "Vegas",
        brightness: 95,
        palette: &[(255, 200, 0), (150, 0, 200), (220, 20, 60), (200, 160, 0)],
    },
    Scene {
        name: "Neon Night",
        brightness: 100,
        palette: &[(255, 10, 120), (10, 220, 255), (130, 255, 0), (255, 0, 200)],
    },
    Scene {
        name: "Carnival",
        brightness: 95,
        palette: &[(255, 0, 0), (255, 200, 0), (0, 100, 255), (0, 180, 0), (200, 0, 200)],
    },
    Scene {
        name: "Beach Party",
        brightness: 90,
        palette: &[(0, 200, 200), (255, 220, 0), (255, 100, 80), (0, 180, 200)],
    },
    Scene {
        name: "Retro",
        brightness: 85,
        palette: &[(255, 100, 30), (0, 180, 150), (255, 220, 0), (200, 80, 20)],
    },
    Scene {
        name: "Pop Art",
        brightness: 100,
        palette: &[(255, 20, 147), (0, 180, 255), (255, 230, 0), (255, 0, 100)],
    },
    Scene {
        name: "Electric Storm",
        brightness: 90,
        palette: &[(200, 200, 255), (50, 100, 255), (150, 0, 200), (100, 150, 255)],
    },
    Scene {
        name: "Pride",
        brightness: 100,
        palette: &[(255, 0, 0), (255, 165, 0), (255, 255, 0), (0, 180, 0), (0, 50, 255), (150, 0, 200)],
    },
    Scene {
        name: "Fiesta",
        brightness: 95,
        palette: &[(255, 30, 30), (255, 200, 0), (0, 180, 80), (200, 0, 60)],
    },
];
