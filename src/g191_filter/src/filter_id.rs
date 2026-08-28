// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

/// ITU-T G.191 filter identifiers (case-insensitive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterId {
    // FIR filters
    HQDown2To1,
    HQDown3To1,
    FlatBandPass,
    IRS8,
    IRS16,
    ModIRS16,
    ModIRS48,
    LP1p5_48k,
    LP35_48k,
    LP7_48k,
    LP10_48k,
    LP12_48k,
    LP14_48k,
    LP20_48k,
    // IIR filters
    G712_8k,
    DirDCRemoval,
    DirLP3To1,
    DirLP1To3,
    CascLP3To1,
    CascLP1To3,
    Unknown,
}

/// Filter type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Fir,
    Iir,
}

impl FilterId {
    /// Convert from STL filter name (case-insensitive)
    pub fn from_stl_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "hq_down_2_to_1" => Self::HQDown2To1,
            "hq_down_3_to_1" => Self::HQDown3To1,
            "flat_band_pass" => Self::FlatBandPass,
            "irs8khz" => Self::IRS8,
            "irs16khz" => Self::IRS16,
            "mod_irs16khz" => Self::ModIRS16,
            "mod_irs48khz" => Self::ModIRS48,
            "lp1p5_48khz" => Self::LP1p5_48k,
            "lp35_48khz" => Self::LP35_48k,
            "lp7_48khz" => Self::LP7_48k,
            "lp10_48khz" => Self::LP10_48k,
            "lp12_48khz" => Self::LP12_48k,
            "lp14_48khz" => Self::LP14_48k,
            "lp20_48khz" => Self::LP20_48k,
            "g712_8khz" => Self::G712_8k,
            "dir_dc_removal" => Self::DirDCRemoval,
            "iir_down_3_to_1" => Self::DirLP3To1,
            "iir_up_1_to_3" => Self::DirLP1To3,
            "iir_casc_lp_3_to_1" => Self::CascLP3To1,
            "iir_casc_lp_1_to_3" => Self::CascLP1To3,
            _ => Self::Unknown,
        }
    }

    /// Get STL filter name
    pub fn stl_name(&self) -> &'static str {
        match self {
            Self::HQDown2To1 => "hq_down_2_to_1",
            Self::HQDown3To1 => "hq_down_3_to_1",
            Self::FlatBandPass => "flat_band_pass",
            Self::IRS8 => "irs8khz",
            Self::IRS16 => "irs16khz",
            Self::ModIRS16 => "mod_irs16khz",
            Self::ModIRS48 => "mod_irs48khz",
            Self::LP1p5_48k => "lp1p5_48khz",
            Self::LP35_48k => "lp35_48khz",
            Self::LP7_48k => "lp7_48khz",
            Self::LP10_48k => "lp10_48khz",
            Self::LP12_48k => "lp12_48khz",
            Self::LP14_48k => "lp14_48khz",
            Self::LP20_48k => "lp20_48khz",
            Self::G712_8k => "g712_8khz",
            Self::DirDCRemoval => "dir_dc_removal",
            Self::DirLP3To1 => "iir_down_3_to_1",
            Self::DirLP1To3 => "iir_up_1_to_3",
            Self::CascLP3To1 => "iir_casc_lp_3_to_1",
            Self::CascLP1To3 => "iir_casc_lp_1_to_3",
            Self::Unknown => "unknown",
        }
    }

    /// Get filter type
    pub fn filter_type(&self) -> FilterType {
        match self {
            Self::HQDown2To1 | Self::HQDown3To1 | Self::FlatBandPass
            | Self::IRS8 | Self::IRS16 | Self::ModIRS16 | Self::ModIRS48
            | Self::LP1p5_48k | Self::LP35_48k | Self::LP7_48k
            | Self::LP10_48k | Self::LP12_48k | Self::LP14_48k | Self::LP20_48k => FilterType::Fir,
            Self::G712_8k | Self::DirDCRemoval | Self::DirLP3To1 | Self::DirLP1To3
            | Self::CascLP3To1 | Self::CascLP1To3 => FilterType::Iir,
            Self::Unknown => FilterType::Fir,
        }
    }
}

impl std::fmt::Display for FilterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stl_name())
    }
}

impl std::str::FromStr for FilterId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = Self::from_stl_name(s);
        if id == Self::Unknown {
            Err(format!("Unknown filter ID: {s}"))
        } else {
            Ok(id)
        }
    }
}
