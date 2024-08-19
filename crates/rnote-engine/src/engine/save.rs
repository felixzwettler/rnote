use serde::{Deserialize, Serialize};

use crate::fileformats::rnoteformat::{
    methods::{CompM, SerM},
    RnoteHeader,
};

// Rnote file save preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "save_prefs")]
pub struct SavePrefs {
    #[serde(rename = "serialization")]
    pub serialization: SerM,
    #[serde(rename = "compression")]
    pub compression: CompM,
}

impl SavePrefs {
    pub fn clone_config(&self) -> Self {
        self.clone()
    }
}

impl Default for SavePrefs {
    fn default() -> Self {
        Self {
            serialization: SerM::default(),
            compression: CompM::default(),
        }
    }
}

impl From<RnoteHeader> for SavePrefs {
    fn from(value: RnoteHeader) -> Self {
        Self {
            serialization: value.serialization,
            compression: value.compression,
        }
    }
}

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum CompressionLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    VeryLow,
    None,
}

impl TryFrom<u32> for CompressionLevel {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value).ok_or_else(|| {
            anyhow::anyhow!(
                "CompressionLevel try_from::<u32>() for value {} failed",
                value
            )
        })
    }
}

impl CompM {
    pub fn get_compression_level(&self) -> CompressionLevel {
        match self {
            Self::None => CompressionLevel::None,
            Self::Gzip(val) => match *val {
                1..=2 => CompressionLevel::VeryLow,
                3..=4 => CompressionLevel::Low,
                5 => CompressionLevel::Medium,
                6..=7 => CompressionLevel::High,
                8..=9 => CompressionLevel::VeryHigh,
                _ => unreachable!(),
            },
            Self::Zstd(val) => match *val {
                1..=4 => CompressionLevel::VeryLow,
                5..=8 => CompressionLevel::Low,
                9..=12 => CompressionLevel::Medium,
                13..=16 => CompressionLevel::High,
                17..=21 => CompressionLevel::VeryHigh,
                _ => unreachable!(),
            },
        }
    }
    pub fn set_compression_level(&mut self, level: CompressionLevel) {
        match self {
            Self::None => (),
            Self::Gzip(ref mut val) => {
                *val = match level {
                    CompressionLevel::VeryHigh => 8,
                    CompressionLevel::High => 6,
                    CompressionLevel::Medium => 5,
                    CompressionLevel::Low => 3,
                    CompressionLevel::VeryLow => 1,
                    CompressionLevel::None => unreachable!(),
                }
            }
            Self::Zstd(ref mut val) => {
                *val = match level {
                    CompressionLevel::VeryHigh => 17,
                    CompressionLevel::High => 13,
                    CompressionLevel::Medium => 9,
                    CompressionLevel::Low => 5,
                    CompressionLevel::VeryLow => 1,
                    CompressionLevel::None => unreachable!(),
                }
            }
        }
    }
}
