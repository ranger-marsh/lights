//! Built-in scene presets — static palette data, no persistence required.
//!
//! Each [`Scene`] carries a colour palette.
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
        palette: &[(34, 139, 34), (0, 100, 0), (144, 238, 144), (0, 128, 0)],
    },
    Scene {
        name: "Ocean",
        palette: &[(0, 119, 190), (0, 206, 209), (64, 164, 223), (0, 150, 200)],
    },
    Scene {
        name: "Sunset",
        palette: &[(255, 94, 77), (255, 154, 0), (200, 80, 192), (255, 70, 30)],
    },
    Scene {
        name: "Sunrise",
        palette: &[(255, 183, 197), (255, 218, 185), (255, 223, 0), (255, 160, 80)],
    },
    Scene {
        name: "Desert",
        palette: &[(210, 180, 140), (205, 92, 92), (255, 140, 0), (194, 144, 80)],
    },
    Scene {
        name: "Aurora",
        palette: &[(0, 255, 127), (75, 0, 130), (0, 191, 255), (100, 220, 180)],
    },
    Scene {
        name: "Firefly",
        palette: &[(180, 255, 0), (255, 220, 0), (100, 200, 50), (200, 255, 80)],
    },
    Scene {
        name: "Coral Reef",
        palette: &[(255, 127, 80), (0, 206, 209), (255, 165, 0), (255, 80, 120)],
    },
    Scene {
        name: "Thunderstorm",
        palette: &[(25, 25, 112), (200, 200, 255), (65, 105, 225), (120, 120, 200)],
    },
    Scene {
        name: "Meadow",
        palette: &[(124, 252, 0), (173, 255, 47), (240, 255, 240), (154, 205, 50)],
    },
    Scene {
        name: "Autumn",
        palette: &[(255, 100, 10), (180, 30, 30), (218, 165, 32), (160, 60, 0)],
    },
    Scene {
        name: "Rainforest",
        palette: &[(0, 80, 0), (50, 205, 50), (34, 139, 34), (0, 160, 60)],
    },
    Scene {
        name: "Tidal Pool",
        palette: &[(0, 160, 180), (80, 220, 200), (0, 100, 140), (40, 200, 180)],
    },
    Scene {
        name: "Wildfire",
        palette: &[(255, 60, 0), (200, 10, 0), (255, 160, 0), (240, 80, 0)],
    },
    Scene {
        name: "Cherry Blossom",
        palette: &[(255, 183, 197), (255, 153, 180), (255, 210, 220), (240, 140, 170)],
    },
    Scene {
        name: "Northern Lights",
        palette: &[(0, 200, 120), (0, 100, 200), (80, 255, 180), (30, 160, 255)],
    },
    Scene {
        name: "Glacier",
        palette: &[(160, 220, 240), (200, 240, 255), (120, 200, 230), (180, 230, 248)],
    },
    Scene {
        name: "Lagoon",
        palette: &[(0, 180, 160), (0, 220, 200), (0, 140, 130), (40, 210, 190)],
    },
];

// ── Holiday ───────────────────────────────────────────────────────────────────

pub static HOLIDAY: &[Scene] = &[
    Scene {
        name: "Christmas",
        palette: &[(220, 20, 60), (0, 128, 0), (220, 20, 60), (0, 160, 0)],
    },
    Scene {
        name: "Halloween",
        palette: &[(255, 100, 0), (128, 0, 128), (255, 80, 0), (100, 0, 100)],
    },
    Scene {
        name: "Valentine's",
        palette: &[(220, 20, 60), (255, 105, 180), (255, 20, 80), (255, 180, 200)],
    },
    Scene {
        name: "4th of July",
        palette: &[(220, 20, 60), (240, 240, 255), (0, 0, 200), (220, 20, 60)],
    },
    Scene {
        name: "Easter",
        palette: &[(255, 182, 193), (221, 160, 221), (152, 251, 152), (255, 255, 153)],
    },
    Scene {
        name: "St. Patrick's",
        palette: &[(0, 128, 0), (0, 201, 87), (128, 255, 0), (0, 160, 60)],
    },
    Scene {
        name: "Hanukkah",
        palette: &[(0, 70, 200), (192, 192, 192), (255, 255, 255), (30, 100, 220)],
    },
    Scene {
        name: "New Year's",
        palette: &[(255, 215, 0), (192, 192, 192), (255, 240, 200), (200, 180, 0)],
    },
    Scene {
        name: "Thanksgiving",
        palette: &[(255, 140, 0), (139, 69, 19), (218, 165, 32), (200, 100, 20)],
    },
    Scene {
        name: "Diwali",
        palette: &[(255, 215, 0), (255, 140, 0), (200, 20, 20), (255, 180, 0)],
    },
    Scene {
        name: "Mardi Gras",
        palette: &[(128, 0, 128), (255, 215, 0), (0, 128, 0), (180, 0, 180)],
    },
    Scene {
        name: "Winter",
        palette: &[(20, 20, 100), (200, 225, 255), (150, 190, 255), (220, 235, 255)],
    },
    Scene {
        name: "Cinco de Mayo",
        palette: &[(200, 0, 50), (0, 160, 50), (240, 200, 0), (180, 0, 40)],
    },
    Scene {
        name: "Lunar New Year",
        palette: &[(220, 20, 20), (255, 200, 0), (200, 0, 0), (255, 215, 30)],
    },
    Scene {
        name: "Bonfire Night",
        palette: &[(255, 80, 0), (200, 20, 0), (255, 160, 0), (240, 60, 0)],
    },
    Scene {
        name: "Oktoberfest",
        palette: &[(0, 80, 180), (255, 200, 0), (0, 60, 160), (220, 175, 0)],
    },
    Scene {
        name: "Eid",
        palette: &[(0, 160, 80), (255, 215, 0), (0, 130, 60), (240, 195, 0)],
    },
    Scene {
        name: "Day of Dead",
        palette: &[(220, 0, 180), (255, 180, 0), (0, 180, 80), (180, 0, 220)],
    },
];

// ── Relax ─────────────────────────────────────────────────────────────────────

pub static RELAX: &[Scene] = &[
    Scene {
        name: "Meditation",
        palette: &[(100, 50, 200), (150, 100, 255), (80, 40, 180), (130, 80, 220)],
    },
    Scene {
        name: "Spa",
        palette: &[(152, 251, 152), (240, 248, 255), (173, 216, 230), (180, 255, 200)],
    },
    Scene {
        name: "Sleep",
        // Raised from 15 → 20 and boosted green slightly so pendant stays on.
        palette: &[(200, 15, 10), (180, 12, 8)],
    },
    Scene {
        name: "Reading",
        palette: &[(255, 230, 180), (255, 220, 160)],
    },
    Scene {
        name: "Movie Night",
        palette: &[(180, 100, 20), (160, 80, 10)],
    },
    Scene {
        name: "Yoga",
        palette: &[(230, 190, 255), (255, 218, 185), (200, 180, 255), (255, 200, 200)],
    },
    Scene {
        name: "Bath Time",
        palette: &[(100, 149, 237), (175, 238, 238), (120, 180, 255), (150, 220, 240)],
    },
    Scene {
        name: "Candlelight",
        palette: &[(255, 147, 41), (220, 100, 20), (240, 130, 30)],
    },
    Scene {
        name: "Wind Down",
        palette: &[(200, 80, 30), (180, 60, 20), (160, 50, 10)],
    },
    Scene {
        name: "Campfire",
        palette: &[(255, 100, 20), (200, 30, 30), (255, 200, 0), (220, 80, 10)],
    },
    Scene {
        name: "Mindfulness",
        palette: &[(138, 43, 226), (0, 139, 139), (120, 40, 200), (0, 120, 120)],
    },
    Scene {
        name: "Cloud Nine",
        palette: &[(230, 240, 255), (200, 210, 255), (220, 220, 255), (210, 225, 255)],
    },
    Scene {
        name: "Tea Time",
        palette: &[(210, 180, 140), (255, 220, 170), (200, 160, 120), (240, 200, 150)],
    },
    Scene {
        name: "Garden",
        palette: &[(60, 180, 60), (255, 200, 80), (220, 120, 160), (80, 200, 100)],
    },
    Scene {
        name: "Rain",
        palette: &[(80, 120, 180), (140, 170, 210), (60, 100, 160), (120, 160, 200)],
    },
    Scene {
        name: "Lofi",
        palette: &[(180, 120, 200), (100, 160, 200), (160, 100, 180), (120, 180, 210)],
    },
    Scene {
        name: "Nap Time",
        palette: &[(120, 80, 160), (100, 60, 140), (140, 90, 170), (110, 70, 150)],
    },
    Scene {
        name: "Stargazing",
        palette: &[(20, 20, 80), (60, 40, 120), (30, 30, 100), (80, 60, 140)],
    },
];

// ── Cosmic ────────────────────────────────────────────────────────────────────

pub static COSMIC: &[Scene] = &[
    Scene {
        name: "Aurora Borealis",
        palette: &[(0, 255, 127), (75, 0, 130), (0, 191, 255), (100, 220, 180)],
    },
    Scene {
        name: "Galaxy",
        palette: &[(75, 0, 130), (25, 25, 112), (255, 20, 147), (100, 0, 160)],
    },
    Scene {
        name: "Nebula",
        palette: &[(255, 0, 200), (0, 200, 255), (148, 0, 211), (200, 0, 255)],
    },
    Scene {
        name: "Supernova",
        palette: &[(255, 255, 200), (255, 200, 0), (255, 100, 0), (255, 240, 180)],
    },
    Scene {
        name: "Milky Way",
        palette: &[(200, 220, 255), (220, 210, 255), (180, 200, 240), (210, 215, 250)],
    },
    Scene {
        name: "Solar Flare",
        palette: &[(255, 140, 0), (255, 220, 0), (220, 40, 0), (255, 180, 20)],
    },
    Scene {
        name: "Black Hole",
        // Raised from 12 → 20: pendant lights cut power below ~15% effective output.
        palette: &[(40, 0, 80), (70, 0, 100), (30, 0, 60)],
    },
    Scene {
        name: "Comet",
        palette: &[(230, 250, 255), (30, 100, 255), (180, 230, 255), (60, 140, 255)],
    },
    Scene {
        name: "Mars",
        palette: &[(180, 50, 30), (220, 80, 20), (150, 30, 10), (200, 60, 20)],
    },
    Scene {
        name: "Moonrise",
        palette: &[(180, 190, 210), (210, 220, 240), (240, 245, 255), (195, 205, 225)],
    },
    Scene {
        name: "Pulsar",
        palette: &[(0, 50, 255), (200, 200, 255), (30, 80, 255), (180, 190, 255)],
    },
    Scene {
        name: "Wormhole",
        palette: &[(0, 80, 100), (100, 0, 150), (0, 100, 200), (80, 20, 140)],
    },
    Scene {
        name: "Event Horizon",
        palette: &[(60, 0, 120), (180, 80, 255), (30, 0, 80), (140, 40, 200)],
    },
    Scene {
        name: "Stardust",
        palette: &[(220, 200, 255), (255, 240, 200), (200, 215, 255), (240, 225, 255)],
    },
    Scene {
        name: "Saturn",
        palette: &[(210, 180, 100), (180, 150, 80), (240, 210, 130), (195, 165, 90)],
    },
    Scene {
        name: "Deep Space",
        palette: &[(10, 0, 40), (40, 0, 100), (20, 0, 60), (60, 10, 120)],
    },
    Scene {
        name: "Jupiter",
        palette: &[(200, 140, 80), (180, 100, 60), (240, 180, 120), (160, 80, 50)],
    },
    Scene {
        name: "Starfield",
        palette: &[(240, 245, 255), (180, 200, 255), (255, 240, 220), (200, 220, 255)],
    },
];

// ── Party ─────────────────────────────────────────────────────────────────────

pub static PARTY: &[Scene] = &[
    Scene {
        name: "Disco",
        palette: &[(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (255, 0, 255)],
    },
    Scene {
        name: "Tropical",
        palette: &[(0, 210, 210), (255, 20, 147), (255, 230, 0), (0, 200, 180)],
    },
    Scene {
        name: "Rave",
        palette: &[(0, 100, 255), (0, 255, 100), (255, 0, 200), (150, 0, 255)],
    },
    Scene {
        name: "Vegas",
        palette: &[(255, 200, 0), (150, 0, 200), (220, 20, 60), (200, 160, 0)],
    },
    Scene {
        name: "Neon Night",
        palette: &[(255, 10, 120), (10, 220, 255), (130, 255, 0), (255, 0, 200)],
    },
    Scene {
        name: "Carnival",
        palette: &[(255, 0, 0), (255, 200, 0), (0, 100, 255), (0, 180, 0), (200, 0, 200)],
    },
    Scene {
        name: "Beach Party",
        palette: &[(0, 200, 200), (255, 220, 0), (255, 100, 80), (0, 180, 200)],
    },
    Scene {
        name: "Retro",
        palette: &[(255, 100, 30), (0, 180, 150), (255, 220, 0), (200, 80, 20)],
    },
    Scene {
        name: "Pop Art",
        palette: &[(255, 20, 147), (0, 180, 255), (255, 230, 0), (255, 0, 100)],
    },
    Scene {
        name: "Electric Storm",
        palette: &[(200, 200, 255), (50, 100, 255), (150, 0, 200), (100, 150, 255)],
    },
    Scene {
        name: "Pride",
        palette: &[(255, 0, 0), (255, 165, 0), (255, 255, 0), (0, 180, 0), (0, 50, 255), (150, 0, 200)],
    },
    Scene {
        name: "Fiesta",
        palette: &[(255, 30, 30), (255, 200, 0), (0, 180, 80), (200, 0, 60)],
    },
    Scene {
        name: "Glow Up",
        palette: &[(255, 0, 160), (255, 200, 0), (0, 240, 180), (200, 0, 255)],
    },
    Scene {
        name: "House Music",
        palette: &[(0, 30, 255), (0, 200, 255), (100, 0, 255), (0, 150, 200)],
    },
    Scene {
        name: "Laser Show",
        palette: &[(255, 0, 0), (0, 255, 200), (255, 0, 200), (0, 200, 255)],
    },
    Scene {
        name: "Confetti",
        palette: &[(255, 60, 60), (60, 255, 60), (60, 60, 255), (255, 255, 0), (255, 60, 255)],
    },
    Scene {
        name: "Arcade",
        palette: &[(255, 0, 80), (0, 255, 80), (80, 0, 255), (255, 200, 0)],
    },
    Scene {
        name: "Foam Party",
        palette: &[(0, 220, 255), (255, 0, 180), (0, 255, 200), (200, 0, 255)],
    },
];
