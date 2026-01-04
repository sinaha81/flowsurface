use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum Period {
    #[default]
    Daily,
    Weekly,
    Monthly,
    VisibleRange,
}

impl std::fmt::Display for Period {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Period::Daily => write!(f, "Daily"),
            Period::Weekly => write!(f, "Weekly"),
            Period::Monthly => write!(f, "Monthly"),
            Period::VisibleRange => write!(f, "Visible Range"),
        }
    }
}

impl Period {
    pub const ALL: [Period; 4] = [
        Period::Daily,
        Period::Weekly,
        Period::Monthly,
        Period::VisibleRange,
    ];

    pub fn to_timeframe(self) -> Option<exchange::Timeframe> {
        match self {
            Period::Daily => Some(exchange::Timeframe::D1),
            Period::Weekly => Some(exchange::Timeframe::W1),
            Period::Monthly => Some(exchange::Timeframe::MN1),
            Period::VisibleRange => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub period: Period,
    pub value_area_pct: f32,
    pub row_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            period: Period::Daily,
            value_area_pct: 68.0,
            row_height: 20.0,
        }
    }
}
