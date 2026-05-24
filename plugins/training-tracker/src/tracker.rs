//! Training facility visit counter state.
//!
//! Tracks how many times each training facility has been selected during a
//! single career run. State resets when a new career is detected.

use std::sync::Mutex;

/// The 5 training facilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Facility {
    Speed = 0,
    Stamina = 1,
    Power = 2,
    Guts = 3,
    Wisdom = 4,
}

impl Facility {
    pub const ALL: [Facility; 5] = [
        Facility::Speed,
        Facility::Stamina,
        Facility::Power,
        Facility::Guts,
        Facility::Wisdom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Facility::Speed => "Speed",
            Facility::Stamina => "Stamina",
            Facility::Power => "Power",
            Facility::Guts => "Guts",
            Facility::Wisdom => "Wit",
        }
    }

    /// Map a `command_id` from the game to a facility index.
    /// Different career scenarios use different command_id ranges but the last
    /// digit consistently maps to the same stat:
    ///   *01 / *1 → Speed (index 0)
    ///   *05 / *2 → Stamina (index 1)
    ///   *02 / *3 → Power (index 2)
    ///   *03 / *4 → Guts (index 3)
    ///   *06 / *5 → Wisdom (index 4)
    ///
    /// See `CommandInfo.ToTrainIndex` in UmamusumeResponseAnalyzer for the
    /// canonical mapping.
    pub fn from_command_id(command_id: i32) -> Option<Facility> {
        // Canonical mappings from UmamusumeResponseAnalyzer
        match command_id {
            // URA / base scenario
            101 | 601 | 1101 => Some(Facility::Speed),
            105 | 602 | 1102 => Some(Facility::Stamina),
            102 | 603 | 1103 => Some(Facility::Power),
            103 | 604 | 1104 => Some(Facility::Guts),
            106 | 605 | 1105 => Some(Facility::Wisdom),

            // UAF (Grand Masters / Venus / Sport)
            2101 | 2201 | 2301 => Some(Facility::Speed),
            2102 | 2202 | 2302 => Some(Facility::Stamina),
            2103 | 2203 | 2303 => Some(Facility::Power),
            2104 | 2204 | 2304 => Some(Facility::Guts),
            2105 | 2205 | 2305 => Some(Facility::Wisdom),

            // Onsen
            901 => Some(Facility::Speed),
            902 => Some(Facility::Power),
            906 => Some(Facility::Wisdom),

            _ => None,
        }
    }
}

/// Per-career tracking state.
pub struct TrackerState {
    /// Hit count per facility. Indexed by `Facility as usize`.
    pub counts: [u32; 5],
    /// The turn number we last saw — used to detect new careers.
    #[allow(dead_code)]
    pub last_turn: i32,
    /// Whether tracking is currently active (in a career).
    pub active: bool,
}

impl TrackerState {
    pub const fn new() -> Self {
        Self {
            counts: [0; 5],
            last_turn: 0,
            active: false,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.counts = [0; 5];
        self.last_turn = 0;
        self.active = false;
    }

    pub fn record_training(&mut self, facility: Facility) {
        self.counts[facility as usize] += 1;
    }

    pub fn total(&self) -> u32 {
        self.counts.iter().sum()
    }
}

pub static TRACKER: Mutex<TrackerState> = Mutex::new(TrackerState::new());
