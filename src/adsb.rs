//! All data structures needed for parsing only [`crate::DF::ADSB`] or [`crate::DF::TisB`]

use deku::bitvec::{BitSlice, Msb0};
use deku::prelude::*;

use std::fmt::Write;

use crate::mode_ac::decode_id13_field;
use crate::{Altitude, CPRFormat, Capability, Sign, ICAO};

/// [`crate::DF::ADSB`] || [`crate::DF::TisB`]
#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct ADSB {
    /// Transponder Capability
    pub capability: Capability,
    /// ICAO aircraft address
    pub icao: ICAO,
    /// Message, extended Squitter
    pub me: ME,
    /// Parity/Interrogator ID
    #[deku(bits = "24")]
    pub pi: u32,
}

impl ADSB {
    /// `to_string` with DF.id() input
    pub(crate) fn to_string(&self, df: u8) -> Result<String, Box<dyn std::error::Error>> {
        let address_type = if df == 17 {
            "(Mode S / ADS-B)"
        } else if df == 18 {
            "(ADS-R)"
        } else {
            unreachable!();
        };

        let mut f = String::new();
        match &self.me {
            ME::AircraftIdentification(Identification { tc, ca, cn }) => {
                writeln!(
                    f,
                    " Extended Squitter Aircraft identification and category (4)"
                )?;
                writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, "  Air/Ground:    {}", self.capability)?;
                writeln!(f, "  Ident:         {}", cn)?;
                writeln!(f, "  Category:      {}{}", tc, ca)?;
            }
            // TODO
            ME::SurfacePosition(..) => (),
            ME::AirbornePositionBaroAltitude(altitude) => {
                writeln!(
                    f,
                    " Extended Squitter Airborne position (barometric altitude) (11)"
                )?;
                writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, "  Air/Ground:    {}", self.capability)?;
                write!(f, "{}", altitude)?;
            }
            ME::AirborneVelocity(airborne_velocity) => {
                if let AirborneVelocitySubType::GroundSpeedDecoding(_) = airborne_velocity.sub_type
                {
                    writeln!(
                        f,
                        " Extended Squitter Airborne velocity over ground, subsonic (19/1)"
                    )?;
                    writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                    writeln!(f, "  Air/Ground:    {}", self.capability)?;
                    writeln!(
                        f,
                        "  GNSS delta:    {}{} ft",
                        airborne_velocity.gnss_sign, airborne_velocity.gnss_baro_diff
                    )?;
                    if let Some((heading, ground_speed, vertical_rate)) =
                        airborne_velocity.calculate()
                    {
                        writeln!(f, "  Heading:       {}", heading.ceil())?;
                        writeln!(
                            f,
                            "  Speed:         {} kt groundspeed",
                            ground_speed.floor()
                        )?;
                        writeln!(
                            f,
                            "  Vertical rate: {} ft/min {}",
                            vertical_rate, airborne_velocity.vrate_src
                        )?;
                    } else {
                        writeln!(f, "  Invalid packet")?;
                    }
                }
            }
            ME::AirbornePositionGNSSAltitude(altitude) => {
                writeln!(
                    f,
                    " Extended Squitter (Non-Transponder) Airborne position (GNSS altitude) (20)"
                )?;
                writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                write!(f, "{}", altitude)?;
            }
            ME::Reserved => (),
            ME::AircraftStatus(AircraftStatus {
                sub_type: _,
                emergency_state: _,
                squawk,
                ..
            }) => {
                writeln!(f, " Extended Squitter Emergency/priority status (28/1)")?;
                writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, "  Air/Ground:    {}", self.capability)?;
                writeln!(f, "  Squawk:        {}", squawk)?;
            }
            ME::TargetStateAndStatusInformation(target_info) => {
                writeln!(f, " Extended Squitter Target state and status (V2) (29/1)")?;
                writeln!(f, "  ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, "  Air/Ground:    {}", self.capability)?;
                writeln!(f, "  Target State and Status:")?;
                writeln!(f, "    Target altitude:   MCP, {} ft", target_info.altitude)?;
                writeln!(f, "    Altimeter setting: {} millibars", target_info.qnh)?;
                if target_info.is_heading {
                    writeln!(f, "    Target heading:    {}", target_info.heading)?;
                }
                if target_info.tcas {
                    write!(f, "    ACAS:              operational")?;
                    if target_info.autopilot {
                        write!(f, " autopilot ")?;
                    }
                    if target_info.vnav {
                        write!(f, " VNAC ")?;
                    }
                    if target_info.alt_hold {
                        write!(f, "altitude-hold ")?;
                    }
                    if target_info.approach {
                        write!(f, "approach")?;
                    }
                    writeln!(f)?;
                } else {
                    writeln!(f, "    ACAS:              NOT operational")?;
                }
                writeln!(f, "    NACp:              {}", target_info.nacp)?;
                writeln!(f, "    NICbaro:           {}", target_info.nicbaro)?;
                writeln!(f, "    SIL:               {} (per sample)", target_info.sil)?;
            }
            ME::AircraftOperationStatus(OperationStatus::Airborne(opstatus_airborne)) => {
                writeln!(
                    f,
                    " Extended Quitter Aircraft operational status (airborne) (31/0)"
                )?;
                writeln!(f, " ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, " Air/Ground:    {}", self.capability)?;
                write!(f, " Aircraft Operational Status:\n{}", opstatus_airborne)?;
            }
            ME::AircraftOperationStatus(OperationStatus::Surface(..)) => {
                writeln!(
                    f,
                    " Extended Quitter Aircraft operational status (surface) (31/0)"
                )?;
                writeln!(f, " ICAO Address:  {} {}", self.icao, address_type)?;
                writeln!(f, " Air/Ground:    {}", self.capability)?;
                write!(f, " Aircraft Operational Status:\nTODO")?;
            }
        }
        Ok(f)
    }
}

/// [`ME::AirborneVelocity`] && [`AirborneVelocitySubType::GroundSpeedDecoding`]
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct GroundSpeedDecoding {
    pub ew_sign: Sign,
    #[deku(endian = "big", bits = "10")]
    pub ew_vel: u16,
    pub ns_sign: Sign,
    #[deku(endian = "big", bits = "10")]
    pub ns_vel: u16,
}

/// [`ME::AirborneVelocity`] && [`AirborneVelocitySubType::AirspeedDecoding`]
#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirspeedDecoding {
    #[deku(bits = "1")]
    pub status_heading: u8,
    #[deku(endian = "big", bits = "10")]
    pub mag_heading: u16,
    #[deku(bits = "1")]
    pub airspeed_type: u8,
    #[deku(endian = "big", bits = "10")]
    pub airspeed: u16,
}

/// [`ME::AircraftOperationStatus`]
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum OperationStatus {
    #[deku(id = "0")]
    Airborne(OperationStatusAirborne),
    #[deku(id = "1")]
    Surface(OperationStatusSurface),
}

/// [`ME::AircraftOperationStatus`] && [`OperationStatus`] == 0
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct OperationStatusAirborne {
    // 16 bits
    pub capability_codes: CapabilityCode,
    #[deku(bits = "5")]
    pub operational_mode_unused1: u8,
    #[deku(bits = "1")]
    pub saf: bool,
    #[deku(bits = "2")]
    pub sda: u8,
    pub operational_mode_unused2: u8,
    pub version_number: ADSBVersion,
    #[deku(bits = "1")]
    pub nic_supplement_a: u8,
    #[deku(bits = "4")]
    pub navigational_accuracy_category: u8,
    #[deku(bits = "2")]
    pub geometric_vertical_accuracy: u8,
    #[deku(bits = "2")]
    pub source_integrity_level: u8,
    #[deku(bits = "1")]
    pub barometric_altitude_integrity: u8,
    #[deku(bits = "1")]
    pub horizontal_reference_direction: u8,
    #[deku(bits = "1")]
    pub sil_supplement: u8,
    #[deku(bits = "1")]
    pub reserved: u8,
}

impl std::fmt::Display for OperationStatusAirborne {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "   Version:            {}", self.version_number)?;
        writeln!(f, "   Capability classes:{}", self.capability_codes)?;
        write!(f, "   Operational modes:  ")?;
        if self.saf {
            write!(f, "SAF ")?;
        }
        if self.sda != 0 {
            write!(f, "SDA={}", self.sda)?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "   NACp:               {}",
            self.navigational_accuracy_category
        )?;
        writeln!(
            f,
            "   GVA:                {}",
            self.geometric_vertical_accuracy
        )?;
        writeln!(
            f,
            "   SIL:                {} (per hour)",
            self.source_integrity_level
        )?;
        writeln!(
            f,
            "   NICbaro:            {}",
            self.barometric_altitude_integrity
        )?;
        if self.horizontal_reference_direction == 1 {
            writeln!(f, "   Heading reference:  magnetic north")?;
        } else {
            writeln!(f, "   Heading reference:  true north")?;
        }
        Ok(())
    }
}

/// [`ME::AircraftOperationStatus`]
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct CapabilityCode {
    #[deku(bits = "2")]
    pub reserved0: u8,
    #[deku(bits = "1")]
    pub acas: u8,
    #[deku(bits = "1")]
    pub cdti: u8,
    #[deku(bits = "2")]
    pub reserved1: u8,
    #[deku(bits = "1")]
    pub arv: u8,
    #[deku(bits = "1")]
    pub ts: u8,
    #[deku(bits = "2")]
    pub tc: u8,
    #[deku(bits = "6")]
    pub reserved2: u8,
}

impl std::fmt::Display for CapabilityCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.acas == 1 {
            write!(f, " ACAS")?;
        }
        if self.cdti == 1 {
            write!(f, " CDTI")?;
        }
        if self.arv == 1 {
            write!(f, " ARV")?;
        }
        if self.ts == 1 {
            write!(f, " TS")?;
        }
        if self.tc == 1 {
            write!(f, " TC")?;
        }
        Ok(())
    }
}

/// [`ME::AircraftOperationStatus`] && [`OperationStatus`] == 1
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct OperationStatusSurface {
    pub capacity_codes: CapabilityCode,
    #[deku(bits = "4")]
    pub capacity_len_code: u8,
    pub operational_mode_codes: u16,
    pub version_number: ADSBVersion,
    #[deku(bits = "1")]
    pub nic_supplement_a: u8,
    #[deku(bits = "4")]
    pub navigational_accuracy_category: u8,
    #[deku(bits = "1")]
    pub reserved0: u8,
    #[deku(bits = "2")]
    pub source_integrity_level: u8,
    #[deku(bits = "1")]
    pub track_angle_or_heading: u8,
    #[deku(bits = "1")]
    pub horizontal_reference_direction: u8,
    #[deku(bits = "1")]
    pub sil_supplement: u8,
    #[deku(bits = "1")]
    pub reserved1: u8,
}

/// ADS-B Defined from different ICAO documents
///
/// reference: ICAO 9871 (5.3.2.3)
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum ADSBVersion {
    #[deku(id = "0b000")]
    DOC9871AppendixA,
    #[deku(id = "0b001")]
    DOC9871AppendixB,
    #[deku(id = "0b010")]
    DOC9871AppendixC,
}

/// Control Field (C.3.3) for [`crate::DF::TisB`]
///
/// reference: ICAO 9871
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(type = "u8", bits = "3")]
#[allow(non_camel_case_types)]
pub enum ControlField {
    /// ADS-B Message from a non-transponder device
    #[deku(id = "0")]
    ADSB_ES_NT(ADSB_ICAO),
    /// Reserved for ADS-B for ES/NT devices for alternate address space
    #[deku(id = "1")]
    ADSB_ES_NT_ALT(ADSB_ICAO),

    ///// Code 2, Fine Format TIS-B Message
    //#[deku(id = "2")]
    //TISB_FINE(TISB_FINE),

    ///// Code 3, Coarse Format TIS-B Message
    //#[deku(id = "3")]
    //TISB_COARSE(TISB_COARSE),

    ///// Code 4, Coarse Format TIS-B Message
    //#[deku(id = "4")]
    //TISB_MANAGE(TISB_MANAGE),

    ///// Code 5, TIS-B Message for replay ADS-B Message
    //#[deku(id = "5")]
    //TISB_ADSB_RELAY(TISB_ADSB_RELAY),
    /// Code 6, TIS-B Message, Same as DF=17
    #[deku(id = "6")]
    TISB_DF17(ADSB),
    ///// Code 7, Reserved
    //#[deku(id = "7")]
    //Reserved,
}

/// [`crate::DF::TisB`] Containing ICAO
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[allow(non_camel_case_types)]
pub struct ADSB_ICAO {
    /// AA: Address, Announced
    aa: ICAO,
    /// ME: message, extended quitter
    #[deku(count = "7")]
    me: Vec<u8>,
    /// PI: Parity/interrogator identifier
    pi: ICAO,
}

/// TODO This should have sort of [ignore] attribute, since we don't need to implement DekuRead on this.
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum Unit {
    Meter = 0,
    Feet  = 1,
}

impl Default for Unit {
    fn default() -> Self {
        Self::Meter
    }
}

/// ADS-B Message, 5 first bits are known as Type Code (TC)
///
/// reference: ICAO 9871 (A.2.3.1)
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(type = "u8", bits = "5")]
pub enum ME {
    #[deku(id_pat = "1..=4")]
    AircraftIdentification(Identification),
    #[deku(id_pat = "5..=8")]
    SurfacePosition(SurfacePosition),
    #[deku(id_pat = "9..=18")]
    AirbornePositionBaroAltitude(Altitude),
    #[deku(id = "19")]
    AirborneVelocity(AirborneVelocity),
    #[deku(id_pat = "20..=22")]
    AirbornePositionGNSSAltitude(Altitude),
    #[deku(id_pat = "23..=27")]
    Reserved,
    #[deku(id = "28")]
    AircraftStatus(AircraftStatus),
    #[deku(id = "29")]
    TargetStateAndStatusInformation(TargetStateAndStatusInformation),
    #[deku(id = "31")]
    AircraftOperationStatus(OperationStatus),
}

/// Table: A-2-97
#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct AircraftStatus {
    pub sub_type: AircraftStatusType,
    pub emergency_state: EmergencyState,
    #[deku(
        bits = "13",
        endian = "big",
        map = "|squawk: u32| -> Result<_, DekuError> {Ok(decode_id13_field(squawk))}"
    )]
    pub squawk: u32,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum AircraftStatusType {
    #[deku(id = "0")]
    NoInformation,
    #[deku(id = "1")]
    EmergencyPriorityStatus,
    #[deku(id_pat = "_")]
    Reserved,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "3")]
pub enum EmergencyState {
    None                 = 0,
    General              = 1,
    Lifeguard            = 2,
    MinimumFuel          = 4,
    UnlawfulInterference = 5,
    Reserved1            = 6,
    Reserved2            = 7,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct OperationCodeSurface {
    #[deku(bits = "1")]
    pub poe: u8,
    #[deku(bits = "1")]
    pub cdti: u8,
    #[deku(bits = "1")]
    pub b2_low: u8,
    #[deku(bits = "3")]
    pub lw: u8,
    #[deku(bits = "6")]
    pub reserved: u16,
}

impl std::fmt::Display for ADSBVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deku_id().unwrap())
    }
}

#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct Identification {
    pub tc: TypeCoding,
    #[deku(bits = "3")]
    pub ca: u8,
    #[deku(reader = "Self::read(deku::rest)")]
    pub cn: String,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "5")]
pub enum TypeCoding {
    D = 1,
    C = 2,
    B = 3,
    A = 4,
}

impl std::fmt::Display for TypeCoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::D => "D",
                Self::C => "C",
                Self::B => "B",
                Self::A => "A",
            }
        )
    }
}

const CHAR_LOOKUP: &[u8; 64] = b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";

impl Identification {
    fn read(rest: &BitSlice<Msb0, u8>) -> Result<(&BitSlice<Msb0, u8>, String), DekuError> {
        let mut inside_rest = rest;

        let mut chars = vec![];
        for _ in 0..=6 {
            let (for_rest, c) = <u8>::read(inside_rest, deku::ctx::Size::Bits(6))?;
            if c != 32 {
                chars.push(c);
            }
            inside_rest = for_rest;
        }
        let encoded = chars
            .into_iter()
            .map(|b| CHAR_LOOKUP[b as usize] as char)
            .collect::<String>();

        Ok((inside_rest, encoded))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
pub struct TargetStateAndStatusInformation {
    // TODO Support V1
    #[deku(bits = "2")]
    pub subtype: u8,
    #[deku(bits = "1")]
    pub is_fms: bool,
    #[deku(
        bits = "12",
        endian = "big",
        map = "|altitude: u32| -> Result<_, DekuError> {Ok(if altitude > 1 {(altitude - 1) * 32} else {0} )}"
    )]
    pub altitude: u32,
    #[deku(
        bits = "9",
        endian = "big",
        map = "|qnh: u32| -> Result<_, DekuError> {if qnh == 0 { Ok(0.0) } else { Ok(800.0 + ((qnh - 1) as f32) * 0.8)}}"
    )]
    pub qnh: f32,
    #[deku(bits = "1")]
    pub is_heading: bool,
    #[deku(
        bits = "9",
        endian = "big",
        map = "|heading: u32| -> Result<_, DekuError> {Ok(heading as f32 * 180.0 / 256.0)}"
    )]
    pub heading: f32,
    #[deku(bits = "4")]
    pub nacp: u8,
    #[deku(bits = "1")]
    pub nicbaro: u8,
    #[deku(bits = "2")]
    pub sil: u8,
    #[deku(bits = "1")]
    pub mode_validity: bool,
    #[deku(bits = "1")]
    pub autopilot: bool,
    #[deku(bits = "1")]
    pub vnav: bool,
    #[deku(bits = "1")]
    pub alt_hold: bool,
    #[deku(bits = "1")]
    pub imf: bool,
    #[deku(bits = "1")]
    pub approach: bool,
    #[deku(bits = "1")]
    pub tcas: bool,
    #[deku(bits = "1")]
    pub lnav: bool,
    #[deku(bits = "2")]
    pub reserved: u8,
}

/// [`ME::AirborneVelocity`]
#[derive(Debug, PartialEq, DekuRead, Clone)]
pub struct AirborneVelocity {
    #[deku(bits = "3")]
    pub st: u8,
    #[deku(bits = "5")]
    pub extra: u8,
    #[deku(ctx = "*st")]
    pub sub_type: AirborneVelocitySubType,
    pub vrate_src: VerticalRateSource,
    pub vrate_sign: Sign,
    #[deku(endian = "big", bits = "9")]
    pub vrate_value: u16,
    #[deku(bits = "2")]
    pub reverved: u8,
    pub gnss_sign: Sign,
    #[deku(
        bits = "7",
        map = "|gnss_baro_diff: u16| -> Result<_, DekuError> {Ok(if gnss_baro_diff > 1 {(gnss_baro_diff - 1)* 25} else { 0 })}"
    )]
    pub gnss_baro_diff: u16,
}

impl AirborneVelocity {
    /// Return effective (heading, ground_speed, vertical_rate)
    pub fn calculate(&self) -> Option<(f64, f64, i16)> {
        if let AirborneVelocitySubType::GroundSpeedDecoding(ground_speed) = &self.sub_type {
            let v_ew = f64::from((ground_speed.ew_vel as i16 - 1) * ground_speed.ew_sign.value());
            let v_ns = f64::from((ground_speed.ns_vel as i16 - 1) * ground_speed.ns_sign.value());
            let h = v_ew.atan2(v_ns) * (360.0 / (2.0 * std::f64::consts::PI));
            let heading = if h < 0.0 { h + 360.0 } else { h };

            let vrate = self
                .vrate_value
                .checked_sub(1)
                .and_then(|v| v.checked_mul(64))
                .map(|v| (v as i16) * self.vrate_sign.value());

            if let Some(vrate) = vrate {
                return Some((heading, v_ew.hypot(v_ns), vrate));
            }
        }
        None
    }
}

/// [`ME::AirborneVelocity`]
#[derive(Debug, PartialEq, DekuRead, Clone)]
#[deku(ctx = "st: u8", id = "st")]
pub enum AirborneVelocitySubType {
    #[deku(id_pat = "1..=2")]
    GroundSpeedDecoding(GroundSpeedDecoding),
    #[deku(id_pat = "3..=4")]
    AirspeedDecoding(AirspeedDecoding),
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "3")]
pub enum AirborneVelocityType {
    Subsonic   = 1,
    Supersonic = 3,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(ctx = "t: AirborneVelocityType")]
pub struct AirborneVelocitySubFields {
    pub dew: DirectionEW,
    #[deku(reader = "Self::read_v(deku::rest, t)")]
    pub vew: u16,
    pub dns: DirectionNS,
    #[deku(reader = "Self::read_v(deku::rest, t)")]
    pub vns: u16,
}

impl AirborneVelocitySubFields {
    fn read_v(
        rest: &BitSlice<Msb0, u8>,
        t: AirborneVelocityType,
    ) -> Result<(&BitSlice<Msb0, u8>, u16), DekuError> {
        match t {
            AirborneVelocityType::Subsonic => {
                u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::Size::Bits(10)))
                    .map(|(rest, value)| (rest, value - 1))
            }
            AirborneVelocityType::Supersonic => {
                u16::read(rest, (deku::ctx::Endian::Big, deku::ctx::Size::Bits(10)))
                    .map(|(rest, value)| (rest, 4 * (value - 1)))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum DirectionEW {
    WestToEast = 0,
    EastToWest = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum DirectionNS {
    SouthToNorth = 0,
    NorthToSouth = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum SourceBitVerticalRate {
    GNSS      = 0,
    Barometer = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum SignBitVerticalRate {
    Up   = 0,
    Down = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, DekuRead)]
#[deku(type = "u8", bits = "1")]
pub enum SignBitGNSSBaroAltitudesDiff {
    Above = 0,
    Below = 1,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum VerticalRateSource {
    BarometricPressureAltitude = 0,
    GeometricAltitude          = 1,
}

impl std::fmt::Display for VerticalRateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::BarometricPressureAltitude => "barometric",
                Self::GeometricAltitude => "GNSS",
            }
        )
    }
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
pub struct SurfacePosition {
    #[deku(bits = "7")]
    pub mov: u8,
    pub s: StatusForGroundTrack,
    #[deku(bits = "7")]
    pub trk: u8,
    #[deku(bits = "1")]
    pub t: bool,
    pub f: CPRFormat,
    #[deku(bits = "17", endian = "big")]
    pub lat_cpr: u32,
    #[deku(bits = "17", endian = "big")]
    pub lon_cpr: u32,
}

#[derive(Debug, PartialEq, DekuRead, Copy, Clone)]
#[deku(type = "u8", bits = "1")]
pub enum StatusForGroundTrack {
    Invalid = 0,
    Valid   = 1,
}