//! This module provides the `Specification` and its associated types to allow interpretation
//! of the fields contained within the `frame_data` payload of `Packet` values.
use std::{cell::RefCell, clone::Clone, fmt, rc::Rc};

use chrono::{DateTime, TimeZone};

use crate::{
    data::Data,
    error::Result,
    packet::{PacketFieldId, PacketId},
    specification_file::{
        Language, PacketTemplateFieldPart, SpecificationFile, Type, Unit, UnitFamily, UnitId,
    },
    utils::utc_timestamp,
};

/// Contains information about a VBus device.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{SpecificationFile, Specification, Language};
///
/// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
///
/// let device_spec = spec.get_device_spec(0x00, 0x7E11, 0x0010);
/// assert_eq!("00_7E11", device_spec.device_id);
/// assert_eq!(0, device_spec.channel);
/// assert_eq!(0x7E11, device_spec.self_address);
/// assert_eq!(None, device_spec.peer_address);
/// assert_eq!("DeltaSol MX [Regler]", device_spec.name);
/// ```
#[derive(Debug)]
pub struct DeviceSpec {
    /// A device identifier.
    pub device_id: String,

    /// The VBus channel the device is attached to.
    pub channel: u8,

    /// The VBus address of the device itself.
    pub self_address: u16,

    /// Optionally the VBus address of the device's peer.
    pub peer_address: Option<u16>,

    /// The name of the device.
    pub name: String,
}

/// Contains information about a VBus packet and its fields.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{SpecificationFile, Specification, Language};
///
/// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
///
/// let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);
/// assert_eq!("00_0010_7E11_10_0100", packet_spec.packet_id);
/// assert_eq!(0, packet_spec.channel);
/// assert_eq!(0x0010, packet_spec.destination_address);
/// assert_eq!(0x7E11, packet_spec.source_address);
/// assert_eq!(0x0100, packet_spec.command);
/// assert_eq!("DFA", packet_spec.destination_device.name);
/// assert_eq!("DeltaSol MX [Regler]", packet_spec.source_device.name);
/// assert_eq!("DeltaSol MX [Regler]", packet_spec.name);
/// ```
#[derive(Debug)]
pub struct PacketSpec {
    /// A packet identifier.
    pub packet_id: String,

    /// The VBus channel to packet was sent to.
    pub channel: u8,

    /// The destination VBus address the packet was sent to.
    pub destination_address: u16,

    /// The source VBus address to packet was send from.
    pub source_address: u16,

    /// The VBus command of the packet.
    pub command: u16,

    /// The `DeviceSpec` containing information about the destination VBus device.
    pub destination_device: Rc<DeviceSpec>,

    /// The `DeviceSpec` containing information about the source VBus device.
    pub source_device: Rc<DeviceSpec>,

    /// The name of the packet, containing channel, source and optionally destination names.
    pub name: String,

    /// The fields contained in the frame payload of the VBus packet.
    pub fields: Vec<PacketFieldSpec>,
}

/// Contains information about a VBus packet field.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{SpecificationFile, Specification, Language};
/// use resol_vbus::specification_file::{UnitFamily, Type};
///
/// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
///
/// let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);
/// let packet_field_spec = &packet_spec.fields [0];
///
/// assert_eq!("000_2_0", packet_field_spec.field_id);
/// assert_eq!("00_0010_7E11_10_0100_000_2_0", packet_field_spec.packet_field_id);
/// assert_eq!("Temperatur Sensor 1", packet_field_spec.name);
/// assert_eq!(62, packet_field_spec.unit_id.0);
/// assert_eq!(UnitFamily::Temperature, packet_field_spec.unit_family);
/// assert_eq!("DegreesCelsius", packet_field_spec.unit_code);
/// assert_eq!(" °C", packet_field_spec.unit_text);
/// assert_eq!(1, packet_field_spec.precision);
/// assert_eq!(Type::Number, packet_field_spec.typ);
/// ```
#[derive(Debug, PartialEq)]
pub struct PacketFieldSpec {
    /// A field identifier.
    pub field_id: String,

    /// A packet-field identifier.
    pub packet_field_id: String,

    /// The name of the field.
    pub name: String,

    /// The `UnitId` of the field.
    pub unit_id: UnitId,

    /// The `UnitFamily` of the field.
    pub unit_family: UnitFamily,

    /// The unit code of the field.
    pub unit_code: String,

    /// The unit text of the field.
    pub unit_text: String,

    /// The precision of the field.
    pub precision: i32,

    /// The `Type` of the field.
    pub typ: Type,

    /// The parts the field consists of.
    pub parts: Vec<PacketTemplateFieldPart>,

    /// The language used for the specification.
    pub language: Language,
}

/// A helper type for formatting raw values.
#[derive(Debug)]
pub struct RawValueFormatter<'a> {
    language: Language,
    typ: Type,
    precision: i32,
    raw_value: i64,
    unit_text: &'a str,
}

/// A helper type for formatting raw values.
#[derive(Debug)]
pub struct PacketFieldFormatter<'a> {
    language: Language,
    typ: Type,
    precision: i32,
    raw_value: Option<i64>,
    unit_text: &'a str,
}

/// The `Specification` type contains information about known devices and packets.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{SpecificationFile, Specification, Language};
///
/// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
///
/// let device_spec = spec.get_device_spec(0x00, 0x7E11, 0x0010);
/// assert_eq!("00_7E11", device_spec.device_id);
/// assert_eq!(0, device_spec.channel);
/// assert_eq!(0x7E11, device_spec.self_address);
/// assert_eq!(None, device_spec.peer_address);
/// assert_eq!("DeltaSol MX [Regler]", device_spec.name);
/// ```
#[derive(Debug)]
pub struct Specification {
    file: SpecificationFile,
    language: Language,
    devices: RefCell<Vec<Rc<DeviceSpec>>>,
    packets: RefCell<Vec<Rc<PacketSpec>>>,
}

/// An iterator over the fields of the `Packet` instances in a `DataSet`.
///
/// The function `Specification::fields_in_data_set` returns this iterator.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{Specification, DataSet};
///
/// # #[allow(dead_code)]
/// fn print_fields(spec: &Specification, data_set: &DataSet) {
///     let mut last_data_index = None;
///     for field in spec.fields_in_data_set(data_set) {
///         let current_data_index = Some(field.data_index());
///         if last_data_index != current_data_index {
///             last_data_index = current_data_index;
///             println!("- {}: {}", field.packet_spec().packet_id, field.packet_spec().name);
///         }
///         println!("    - {}: {}", field.field_spec().field_id, field.field_spec().name);
///     }
/// }
/// ```
#[derive(Debug)]
pub struct DataSetPacketFieldIterator<'a, T: AsRef<[Data]>> {
    spec: &'a Specification,
    data_set: &'a T,
    data_index: usize,
    field_index: usize,
}

/// An item returned from the `DataSetPacketFieldIterator` for each field.
#[derive(Debug)]
pub struct DataSetPacketField<'a, T: AsRef<[Data]>> {
    data_set: &'a T,
    data_index: usize,
    packet_spec: Rc<PacketSpec>,
    field_index: usize,
    raw_value: Option<i64>,
}

fn get_cached_device_spec(
    devices: &[Rc<DeviceSpec>],
    channel: u8,
    self_address: u16,
    peer_address: u16,
) -> Option<Rc<DeviceSpec>> {
    let peer_address = Some(peer_address);

    let result = devices.iter().find(|&device| {
        device.channel == channel
            && device.self_address == self_address
            && (device.peer_address.is_none() || device.peer_address == peer_address)
    });

    result.cloned()
}

fn get_or_create_cached_device_spec(
    devices: &mut Vec<Rc<DeviceSpec>>,
    channel: u8,
    self_address: u16,
    peer_address: u16,
    file: &SpecificationFile,
    language: Language,
) -> Rc<DeviceSpec> {
    if let Some(device) = get_cached_device_spec(devices, channel, self_address, peer_address) {
        return device;
    }

    let device_template = file.find_device_template(self_address, peer_address);

    let peer_address_option = match device_template {
        None => None,
        Some(device_template) => {
            if device_template.peer_mask == 0 {
                None
            } else {
                Some(peer_address)
            }
        }
    };

    let device_id = match peer_address_option {
        None => format!("{channel:02X}_{self_address:04X}"),
        Some(peer_address) => format!("{channel:02X}_{self_address:04X}_{peer_address:04X}"),
    };

    let name = match device_template {
        None => {
            match language {
                Language::En => format!("Unknown device 0x{self_address:04X}"),
                Language::De => format!("Unbekanntes Gerät 0x{self_address:04X}"),
                Language::Fr => format!("Unknown device 0x{self_address:04X}"), // FIXME(daniel): missing translation
            }
        }
        Some(device_template) => file
            .localized_text_by_index(&device_template.name_localized_text_index, language)
            .to_owned(),
    };

    let name = match channel {
        0 => name,
        _ => format!("VBus {channel}: {name}"),
    };

    let device = DeviceSpec {
        device_id,
        channel,
        self_address,
        peer_address: peer_address_option,
        name,
    };

    devices.push(Rc::new(device));

    get_cached_device_spec(devices, channel, self_address, peer_address).unwrap()
}

fn get_cached_packet_spec(
    packets: &[Rc<PacketSpec>],
    packet_id: PacketId,
) -> Option<Rc<PacketSpec>> {
    let PacketId(channel, destination_address, source_address, command) = packet_id;

    let result = packets.iter().find(|&packet| {
        packet.channel == channel
            && packet.destination_address == destination_address
            && packet.source_address == source_address
            && packet.command == command
    });

    result.cloned()
}

fn get_or_create_cached_packet_spec(
    packets: &mut Vec<Rc<PacketSpec>>,
    packet_id: PacketId,
    devices: &mut Vec<Rc<DeviceSpec>>,
    file: &SpecificationFile,
    language: Language,
) -> Rc<PacketSpec> {
    let PacketId(channel, destination_address, source_address, command) = packet_id;

    if let Some(packet) = get_cached_packet_spec(packets, packet_id) {
        return packet;
    }

    let destination_device = get_or_create_cached_device_spec(
        devices,
        channel,
        destination_address,
        source_address,
        file,
        language,
    );
    let source_device = get_or_create_cached_device_spec(
        devices,
        channel,
        source_address,
        destination_address,
        file,
        language,
    );

    let packet_id_string = packet_id.packet_id_string();

    let packet_name = match destination_address {
        0x0010 => source_device.name.clone(),
        _ => format!("{} => {}", source_device.name, destination_device.name),
    };

    let fields = match file.find_packet_template(destination_address, source_address, command) {
        None => Vec::new(),
        Some(packet_template) => packet_template
            .fields
            .iter()
            .map(|field| {
                let field_id = file.text_by_index(&field.id_text_index).to_string();

                let packet_field_id = format!("{packet_id_string}_{field_id}");

                let field_name = file
                    .localized_text_by_index(&field.name_localized_text_index, language)
                    .to_string();

                let unit = file.unit_by_id(&field.unit_id);

                let unit_family = file.unit_family_by_id(&unit.unit_family_id);
                let unit_code = file.text_by_index(&unit.unit_code_text_index).to_string();
                let unit_text = file.text_by_index(&unit.unit_text_text_index).to_string();

                let typ = file.type_by_id(&field.type_id);

                PacketFieldSpec {
                    field_id,
                    packet_field_id,
                    name: field_name,
                    unit_id: field.unit_id,
                    unit_family,
                    unit_code,
                    unit_text,
                    precision: field.precision,
                    typ,
                    parts: field.parts.clone(),
                    language,
                }
            })
            .collect(),
    };

    let packet = PacketSpec {
        packet_id: packet_id_string,
        channel,
        destination_address,
        source_address,
        command,
        destination_device,
        source_device,
        name: packet_name,
        fields,
    };

    packets.push(Rc::new(packet));

    get_cached_packet_spec(packets, packet_id).unwrap()
}

/// Get the "power of 10" `i64` value for common "n"s and calculate it otherwise.
pub fn power_of_ten_i64(n: u32) -> i64 {
    match n {
        0 => 1,
        1 => 10,
        2 => 100,
        3 => 1_000,
        4 => 10_000,
        5 => 100_000,
        6 => 1_000_000,
        7 => 10_000_000,
        8 => 100_000_000,
        9 => 1_000_000_000,
        _ => 10i64.pow(n),
    }
}

/// Get the "power of 10" `f64` value for common "n"s and calculate it otherwise.
pub fn power_of_ten_f64(n: i32) -> f64 {
    match n {
        -9 => 0.000_000_001,
        -8 => 0.000_000_01,
        -7 => 0.000_000_1,
        -6 => 0.000_001,
        -5 => 0.000_01,
        -4 => 0.000_1,
        -3 => 0.001,
        -2 => 0.01,
        -1 => 0.1,
        0 => 1.0,
        1 => 10.0,
        2 => 100.0,
        3 => 1_000.0,
        4 => 10_000.0,
        5 => 100_000.0,
        6 => 1_000_000.0,
        7 => 10_000_000.0,
        8 => 100_000_000.0,
        9 => 1_000_000_000.0,
        _ => 10.0f64.powf(f64::from(n)),
    }
}

impl Specification {
    /// Construct a `Specification` from a `SpecificationFile` and a `Language`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
    ///
    /// // work with the spec...
    /// # drop(spec);
    /// ```
    pub fn from_file(file: SpecificationFile, language: Language) -> Specification {
        let devices = RefCell::new(Vec::new());
        let packets = RefCell::new(Vec::new());

        Specification {
            file,
            language,
            devices,
            packets,
        }
    }

    /// Get the `SpecificationFile` that was used to construct this `Specification`.
    pub fn specification_file(&self) -> &SpecificationFile {
        &self.file
    }

    /// Get the `Language` that was used to construct this `Specification`.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Get a `DeviceSpec`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
    ///
    /// let device_spec = spec.get_device_spec(0x00, 0x7E11, 0x0010);
    /// assert_eq!("00_7E11", device_spec.device_id);
    /// assert_eq!(0, device_spec.channel);
    /// assert_eq!(0x7E11, device_spec.self_address);
    /// assert_eq!(None, device_spec.peer_address);
    /// assert_eq!("DeltaSol MX [Regler]", device_spec.name);
    /// ```
    pub fn get_device_spec(
        &self,
        channel: u8,
        self_address: u16,
        peer_address: u16,
    ) -> Rc<DeviceSpec> {
        let mut devices = self.devices.borrow_mut();
        get_or_create_cached_device_spec(
            &mut devices,
            channel,
            self_address,
            peer_address,
            &self.file,
            self.language,
        )
    }

    /// Get a `PacketSpec`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
    ///
    /// let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);
    /// assert_eq!("00_0010_7E11_10_0100", packet_spec.packet_id);
    /// assert_eq!(0, packet_spec.channel);
    /// assert_eq!(0x0010, packet_spec.destination_address);
    /// assert_eq!(0x7E11, packet_spec.source_address);
    /// assert_eq!(0x0100, packet_spec.command);
    /// assert_eq!("DFA", packet_spec.destination_device.name);
    /// assert_eq!("DeltaSol MX [Regler]", packet_spec.source_device.name);
    /// assert_eq!("DeltaSol MX [Regler]", packet_spec.name);
    /// ```
    pub fn get_packet_spec(
        &self,
        channel: u8,
        destination_address: u16,
        source_address: u16,
        command: u16,
    ) -> Rc<PacketSpec> {
        let mut devices = self.devices.borrow_mut();
        let mut packets = self.packets.borrow_mut();
        let packet_id = PacketId(channel, destination_address, source_address, command);
        get_or_create_cached_packet_spec(
            &mut packets,
            packet_id,
            &mut devices,
            &self.file,
            self.language,
        )
    }

    /// Get a `PacketSpec`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language, PacketId};
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::De);
    ///
    /// let packet_spec = spec.get_packet_spec_by_id(PacketId(0x00, 0x0010, 0x7E11, 0x0100));
    /// assert_eq!("00_0010_7E11_10_0100", packet_spec.packet_id);
    /// assert_eq!(0, packet_spec.channel);
    /// assert_eq!(0x0010, packet_spec.destination_address);
    /// assert_eq!(0x7E11, packet_spec.source_address);
    /// assert_eq!(0x0100, packet_spec.command);
    /// assert_eq!("DFA", packet_spec.destination_device.name);
    /// assert_eq!("DeltaSol MX [Regler]", packet_spec.source_device.name);
    /// assert_eq!("DeltaSol MX [Regler]", packet_spec.name);
    /// ```
    pub fn get_packet_spec_by_id(&self, packet_id: PacketId) -> Rc<PacketSpec> {
        self.get_packet_spec(packet_id.0, packet_id.1, packet_id.2, packet_id.3)
    }

    /// Returns an iterator that iterates over all known packet fields in the data set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{Specification, DataSet};
    ///
    /// # #[allow(dead_code)]
    /// fn print_fields(spec: &Specification, data_set: &DataSet) {
    ///     let mut last_data_index = None;
    ///     for field in spec.fields_in_data_set(data_set) {
    ///         let current_data_index = Some(field.data_index());
    ///         if last_data_index != current_data_index {
    ///             last_data_index = current_data_index;
    ///             println!("- {}: {}", field.packet_spec().packet_id, field.packet_spec().name);
    ///         }
    ///         println!("    - {}: {}", field.field_spec().field_id, field.field_spec().name);
    ///     }
    /// }
    /// ```
    pub fn fields_in_data_set<'a, T: AsRef<[Data]> + 'a>(
        &'a self,
        data_set: &'a T,
    ) -> DataSetPacketFieldIterator<'a, T> {
        DataSetPacketFieldIterator {
            spec: self,
            data_set,
            data_index: 0,
            field_index: 0,
        }
    }

    /// Format a timestamp.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    /// use resol_vbus::utils::utc_timestamp;
    ///
    /// let fmt_localized_timestamp = |language| {
    ///     let spec = Specification::from_file(SpecificationFile::new_default(), language);
    ///
    ///     format!("{}", spec.fmt_timestamp(&utc_timestamp(1485688933)))
    /// };
    ///
    /// assert_eq!("29/01/2017 11:22:13", fmt_localized_timestamp(Language::En));
    /// assert_eq!("29.01.2017 11:22:13", fmt_localized_timestamp(Language::De));
    /// assert_eq!("29/01/2017 11:22:13", fmt_localized_timestamp(Language::Fr));
    /// ```
    pub fn fmt_timestamp<Tz: TimeZone>(&self, timestamp: &DateTime<Tz>) -> RawValueFormatter<'_> {
        RawValueFormatter {
            language: self.language,
            typ: Type::DateTime,
            precision: 0,
            raw_value: timestamp.naive_local().timestamp() - 978_307_200,
            unit_text: "",
        }
    }

    /// Get `Unit` by its unit code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    /// use resol_vbus::specification_file::UnitId;
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);
    ///
    /// assert_eq!(UnitId(62), spec.unit_by_unit_code("DegreesCelsius").unwrap().unit_id);
    /// assert!(spec.unit_by_unit_code("SomeUnknownUnitCode").is_none());
    /// ```
    pub fn unit_by_unit_code(&self, unit_code: &str) -> Option<&Unit> {
        self.file.unit_by_unit_code(unit_code)
    }

    /// Convert a value from one `Unit` to another.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use resol_vbus::{SpecificationFile, Specification, Language};
    ///
    /// let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);
    ///
    /// let src_unit = spec.unit_by_unit_code("DegreesCelsius").unwrap();
    /// let dst_unit = spec.unit_by_unit_code("DegreesFahrenheit").unwrap();
    /// assert_eq!(Ok(32.0), spec.convert_value(0.0, src_unit, dst_unit));
    /// ```
    pub fn convert_value(&self, value: f64, src_unit: &Unit, dst_unit: &Unit) -> Result<f64> {
        self.file.convert_value(value, src_unit, dst_unit)
    }
}

impl PacketSpec {
    /// Get the position of a `PacketFieldSpec` by its field ID.
    pub fn get_field_spec_position(&self, id: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|field_spec| field_spec.field_id == id)
    }

    /// Get a `PacketFieldSpec` by its position.
    pub fn get_field_spec_by_position(&self, pos: usize) -> &PacketFieldSpec {
        &self.fields[pos]
    }

    /// Get a `PacketFieldSpec` by its field ID.
    pub fn get_field_spec(&self, id: &str) -> Option<&PacketFieldSpec> {
        self.fields
            .iter()
            .find(|field_spec| field_spec.field_id == id)
    }
}

impl PacketFieldSpec {
    /// Construct an `i64` raw value from a slice of bytes.
    pub fn raw_value_i64(&self, buf: &[u8]) -> Option<i64> {
        let length = buf.len();

        let mut valid = false;
        let mut raw_value = 0;

        for part in &self.parts {
            let offset = part.offset as usize;

            if offset < length {
                let mut part_value = if part.is_signed {
                    i64::from(buf[offset] as i8)
                } else {
                    i64::from(buf[offset])
                };
                if part.mask != 0xFF {
                    part_value &= i64::from(part.mask);
                }
                if part.bit_pos > 0 {
                    part_value >>= part.bit_pos;
                }
                raw_value += part_value * part.factor;
                valid = true;
            }
        }

        if valid {
            Some(raw_value)
        } else {
            None
        }
    }

    /// Construct a `f64` raw value from a slice of bytes.
    pub fn raw_value_f64(&self, buf: &[u8]) -> Option<f64> {
        self.raw_value_i64(buf)
            .map(|raw_value| raw_value as f64 * power_of_ten_f64(-self.precision))
    }

    /// Format a raw value into its textual representation.
    pub fn fmt_raw_value(
        &self,
        raw_value: Option<i64>,
        append_unit: bool,
    ) -> PacketFieldFormatter<'_> {
        let unit_text = if append_unit { &self.unit_text } else { "" };
        PacketFieldFormatter {
            language: self.language,
            typ: self.typ,
            precision: self.precision,
            raw_value,
            unit_text,
        }
    }
}

const WEEKDAYS_EN: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

const WEEKDAYS_DE: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

const WEEKDAYS_FR: [&str; 7] = ["Lu", "Ma", "Me", "Je", "Ve", "Sa", "Di"];

impl<'a> RawValueFormatter<'a> {
    /// Construct a `RawValueFormatter` to help format a raw value into its textual representation.
    pub fn new(
        language: Language,
        typ: Type,
        precision: i32,
        raw_value: i64,
        unit_text: &'a str,
    ) -> RawValueFormatter<'a> {
        RawValueFormatter {
            language,
            typ,
            precision,
            raw_value,
            unit_text,
        }
    }
}

impl<'a> fmt::Display for RawValueFormatter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.typ {
            Type::Number => {
                if self.precision > 0 {
                    let sign = if self.raw_value < 0 { "-" } else { "" };
                    let raw_value = self.raw_value.abs();
                    let factor = power_of_ten_i64(self.precision as u32);
                    let left_part = raw_value / factor;
                    let right_part = raw_value % factor;
                    let separator = match self.language {
                        Language::En => ".",
                        Language::De | Language::Fr => ",",
                    };

                    write!(f, "{sign}{left_part}{separator}")?;
                    match self.precision {
                        1 => write!(f, "{right_part:01}")?,
                        2 => write!(f, "{right_part:02}")?,
                        3 => write!(f, "{right_part:03}")?,
                        4 => write!(f, "{right_part:04}")?,
                        5 => write!(f, "{right_part:05}")?,
                        6 => write!(f, "{right_part:06}")?,
                        7 => write!(f, "{right_part:07}")?,
                        8 => write!(f, "{right_part:08}")?,
                        9 => write!(f, "{right_part:09}")?,
                        _ => {
                            let s = format!("{}", right_part + factor);
                            write!(f, "{}", &s[1..])?;
                        }
                    };
                    write!(f, "{}", self.unit_text)
                } else {
                    write!(f, "{}{}", self.raw_value, self.unit_text)
                }
            }
            Type::Time => {
                let hours = self.raw_value / 60;
                let minutes = self.raw_value % 60;
                write!(f, "{hours:02}:{minutes:02}")
            }
            Type::WeekTime => {
                let weekday_idx = ((self.raw_value / 1440) % 7) as usize;
                let hours = (self.raw_value / 60) % 24;
                let minutes = self.raw_value % 60;
                match self.language {
                    Language::En => write!(
                        f,
                        "{},{:02}:{:02}",
                        WEEKDAYS_EN[weekday_idx], hours, minutes
                    ),
                    Language::De => write!(
                        f,
                        "{},{:02}:{:02}",
                        WEEKDAYS_DE[weekday_idx], hours, minutes
                    ),
                    Language::Fr => write!(
                        f,
                        "{},{:02}:{:02}",
                        WEEKDAYS_FR[weekday_idx], hours, minutes
                    ),
                }
            }
            Type::DateTime => {
                let timestamp = utc_timestamp(self.raw_value + 978_307_200);
                match self.language {
                    Language::En | Language::Fr => {
                        write!(f, "{}", timestamp.format("%d/%m/%Y %H:%M:%S"))
                    }
                    Language::De => write!(f, "{}", timestamp.format("%d.%m.%Y %H:%M:%S")),
                }
            }
        }
    }
}

impl<'a> fmt::Display for PacketFieldFormatter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(raw_value) = self.raw_value {
            let formatter = RawValueFormatter::new(
                self.language,
                self.typ,
                self.precision,
                raw_value,
                self.unit_text,
            );
            formatter.fmt(f)
        } else {
            Ok(())
        }
    }
}

impl<'a, T: AsRef<[Data]> + 'a> Iterator for DataSetPacketFieldIterator<'a, T> {
    type Item = DataSetPacketField<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let data_slice = self.data_set.as_ref();
        let data_slice_len = data_slice.len();

        while self.data_index < data_slice_len {
            let data = &data_slice[self.data_index];
            if let Data::Packet(ref packet) = *data {
                let packet_spec = self.spec.get_packet_spec(
                    packet.header.channel,
                    packet.header.destination_address,
                    packet.header.source_address,
                    packet.command,
                );
                if self.field_index < packet_spec.fields.len() {
                    let field_index = self.field_index;
                    self.field_index += 1;

                    let frame_data = &packet.frame_data[0..packet.frame_count as usize * 4];

                    let field_spec = &packet_spec.fields[field_index];
                    let raw_value = field_spec.raw_value_i64(frame_data);

                    return Some(DataSetPacketField {
                        data_set: self.data_set,
                        data_index: self.data_index,
                        packet_spec: packet_spec.clone(),
                        field_index,
                        raw_value,
                    });
                }
            }

            self.data_index += 1;
            self.field_index = 0;
        }

        None
    }
}

impl<'a, T: AsRef<[Data]>> DataSetPacketField<'a, T> {
    /// Construct new `DataSetPacketField` value.
    pub fn new(
        data_set: &'a T,
        data_index: usize,
        packet_spec: Rc<PacketSpec>,
        field_index: usize,
        raw_value: Option<i64>,
    ) -> DataSetPacketField<'a, T> {
        DataSetPacketField {
            data_set,
            data_index,
            packet_spec,
            field_index,
            raw_value,
        }
    }

    /// Return the `DataSet` associated with this field.
    pub fn data_set(&self) -> &[Data] {
        self.data_set.as_ref()
    }

    /// Return the index of the `Data` associated with this field.
    pub fn data_index(&self) -> usize {
        self.data_index
    }

    /// Return the `Data` associated with this field.
    pub fn data(&self) -> &Data {
        &self.data_set.as_ref()[self.data_index]
    }

    /// Return the `PacketSpec` associated with this field.
    pub fn packet_spec(&self) -> &PacketSpec {
        self.packet_spec.as_ref()
    }

    /// Return the index of the `PacketFieldSpec` associated with this field.
    pub fn field_index(&self) -> usize {
        self.field_index
    }

    /// Return the `PacketFieldSpec` associated with this field.
    pub fn field_spec(&self) -> &PacketFieldSpec {
        &self.packet_spec.fields[self.field_index]
    }

    /// Return the `PacketId` associated with this field.
    pub fn packet_id(&self) -> PacketId {
        self.data().as_packet().packet_id()
    }

    /// Return the field ID associated with this field.
    pub fn field_id(&self) -> &str {
        &self.field_spec().field_id
    }

    /// Return the `PacketFieldId` associated with this field.
    pub fn packet_field_id(&self) -> PacketFieldId<'_> {
        PacketFieldId(
            self.data().as_packet().packet_id(),
            &self.field_spec().field_id,
        )
    }

    /// Return the raw value associated with this field.
    pub fn raw_value_i64(&self) -> &Option<i64> {
        &self.raw_value
    }

    /// Return the raw value associated with this field.
    pub fn raw_value_f64(&self) -> Option<f64> {
        self.raw_value
            .map(|v| v as f64 * power_of_ten_f64(-self.field_spec().precision))
    }

    /// Format the raw value associated with this field.
    pub fn fmt_raw_value(&self, append_unit: bool) -> PacketFieldFormatter<'_> {
        self.field_spec().fmt_raw_value(self.raw_value, append_unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        data_set::DataSet,
        recording_reader::RecordingReader,
        test_data::{RECORDING_2, SPEC_FILE_1},
        test_utils::{test_debug_derive, test_partial_eq_derive},
        Header, Packet,
    };

    #[test]
    fn test_device_spec_derived_impls() {
        let ds = DeviceSpec {
            device_id: "DeviceID".into(),
            channel: 0,
            self_address: 0x1234,
            peer_address: None,
            name: "Name".into(),
        };

        test_debug_derive(&ds);
    }

    #[test]
    fn test_packet_spec_derived_impls() {
        let ds = Rc::new(DeviceSpec {
            device_id: "DeviceID".into(),
            channel: 0,
            self_address: 0x1234,
            peer_address: None,
            name: "DeviceName".into(),
        });

        let ps = PacketSpec {
            packet_id: "PacketID".into(),
            channel: 0,
            destination_address: 0x1234,
            source_address: 0x2345,
            command: 0x3456,
            destination_device: ds.clone(),
            source_device: ds,
            name: "Name".into(),
            fields: Vec::new(),
        };

        test_debug_derive(&ps);
    }

    #[test]
    fn test_packet_field_spec_derived_impls() {
        let pfs = PacketFieldSpec {
            field_id: "FieldID".into(),
            packet_field_id: "PacketFieldID".into(),
            name: "Name".into(),
            unit_id: UnitId(0),
            unit_family: UnitFamily::None,
            unit_code: "UnitCode".into(),
            unit_text: "UnitText".into(),
            precision: 0,
            typ: Type::Number,
            parts: Vec::new(),
            language: Language::En,
        };

        test_debug_derive(&pfs);
        test_partial_eq_derive(&pfs);
    }

    #[test]
    fn test_power_of_ten_i64() {
        for n in 0..19 {
            assert_eq!(10i64.pow(n), power_of_ten_i64(n));
        }
    }

    #[test]
    fn test_power_of_ten_f64() {
        for n in -20..20 {
            assert_eq!(10.0f64.powf(n as f64), power_of_ten_f64(n));
        }
    }

    #[test]
    fn test_raw_value_formatter() {
        use crate::specification_file::{Language::*, Type::*};

        let formatter = RawValueFormatter::new(En, Number, 0, 0, "");
        test_debug_derive(&formatter);

        let fmt_to_string = |language, typ, prec, value, unit| {
            let formatter = RawValueFormatter::new(language, typ, prec, value, unit);
            format!("{}", formatter)
        };

        assert_eq!("12346", fmt_to_string(En, Number, 0, 12346, ""));
        assert_eq!("12346 unit", fmt_to_string(En, Number, 0, 12346, " unit"));
        assert_eq!("12345.7", fmt_to_string(En, Number, 1, 123457, ""));
        assert_eq!("12345.68", fmt_to_string(En, Number, 2, 1234568, ""));
        assert_eq!("12345.679", fmt_to_string(En, Number, 3, 12345679, ""));
        assert_eq!("12345.6789", fmt_to_string(En, Number, 4, 123456789, ""));
        assert_eq!(
            "1.2345678900",
            fmt_to_string(En, Number, 10, 12345678900, "")
        );
        assert_eq!(
            "1,2345678900",
            fmt_to_string(De, Number, 10, 12345678900, "")
        );
        assert_eq!(
            "1,2345678900",
            fmt_to_string(Fr, Number, 10, 12345678900, "")
        );

        assert_eq!(
            "12:01",
            fmt_to_string(En, Time, 10, 721, " ignore this unit")
        );
        assert_eq!(
            "12:01",
            fmt_to_string(De, Time, 10, 721, " ignore this unit")
        );
        assert_eq!(
            "12:01",
            fmt_to_string(Fr, Time, 10, 721, " ignore this unit")
        );

        assert_eq!(
            "Th,12:01",
            fmt_to_string(En, WeekTime, 10, 3 * 1440 + 721, " ignore this unit")
        );
        assert_eq!(
            "Do,12:01",
            fmt_to_string(De, WeekTime, 10, 3 * 1440 + 721, " ignore this unit")
        );
        assert_eq!(
            "Je,12:01",
            fmt_to_string(Fr, WeekTime, 10, 3 * 1440 + 721, " ignore this unit")
        );

        assert_eq!(
            "12/22/2013 15:17:42",
            fmt_to_string(En, DateTime, 10, 409418262, " ignore this unit")
        );
        assert_eq!(
            "22.12.2013 15:17:42",
            fmt_to_string(De, DateTime, 10, 409418262, " ignore this unit")
        );
        assert_eq!(
            "22/12/2013 15:17:42",
            fmt_to_string(Fr, DateTime, 10, 409418262, " ignore this unit")
        );
    }

    #[test]
    fn test_packet_field_formatter_derived_impls() {
        let pff = PacketFieldFormatter {
            language: Language::En,
            typ: Type::Number,
            precision: 0,
            raw_value: None,
            unit_text: "UnitText",
        };

        test_debug_derive(&pff);
    }

    #[test]
    fn test_specification_derived_impls() {
        let spec = Specification {
            file: SpecificationFile::new_default(),
            language: Language::En,
            devices: RefCell::new(Vec::new()),
            packets: RefCell::new(Vec::new()),
        };

        test_debug_derive(&spec);
    }

    #[test]
    fn test_data_set_packet_field_iterator_derived_impls() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        let data_set = DataSet::new();

        let it = DataSetPacketFieldIterator {
            spec: &spec,
            data_set: &data_set,
            data_index: 0,
            field_index: 0,
        };

        test_debug_derive(&it);
    }

    #[test]
    fn test_data_set_packet_field_derived_impls() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        let data_set = DataSet::new();

        let packet_spec = spec.get_packet_spec(0, 0x0010, 0x7E11, 0x0100);

        let dspf = DataSetPacketField {
            data_set: &data_set,
            data_index: 0,
            packet_spec: packet_spec,
            field_index: 0,
            raw_value: None,
        };

        test_debug_derive(&dspf);
    }

    #[test]
    fn test_get_or_create_cached_device_spec() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let mut devices = Vec::new();

        let device_spec = get_or_create_cached_device_spec(
            &mut devices,
            0x01,
            0xFFFF,
            0x0010,
            &spec_file,
            Language::En,
        );

        assert_eq!("VBus 1: Unknown device 0xFFFF", &device_spec.name);

        let device_spec = get_or_create_cached_device_spec(
            &mut devices,
            0x02,
            0xFFFF,
            0x0010,
            &spec_file,
            Language::De,
        );

        assert_eq!("VBus 2: Unbekanntes Gerät 0xFFFF", &device_spec.name);

        let device_spec = get_or_create_cached_device_spec(
            &mut devices,
            0x03,
            0xFFFF,
            0x0010,
            &spec_file,
            Language::Fr,
        );

        assert_eq!("VBus 3: Unknown device 0xFFFF", &device_spec.name); // FIXME(daniel): fix translation and test
    }

    #[test]
    fn test_from_file() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.devices.borrow().len());
        assert_eq!(0, spec.packets.borrow().len());
    }

    #[test]
    fn test_specification_file() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        assert_eq!(20161007, spec_file.datecode);

        let spec = Specification::from_file(spec_file, Language::En);

        let spec_file = spec.specification_file();

        assert_eq!(20161007, spec_file.datecode);
    }

    #[test]
    fn test_language() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(Language::En, spec.language());
    }

    #[test]
    fn test_get_device_spec() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.devices.borrow().len());

        let device_spec = spec.get_device_spec(0x01, 0x7E31, 0x0010);

        assert_eq!(1, spec.devices.borrow().len());
        assert_eq!("01_7E31", device_spec.device_id);
        assert_eq!(0x01, device_spec.channel);
        assert_eq!(0x7E31, device_spec.self_address);
        assert_eq!(None, device_spec.peer_address);
        assert_eq!("VBus 1: DeltaSol MX [WMZ #1]", device_spec.name);

        let device_spec = spec.get_device_spec(0x01, 0x7E31, 0x0010);

        assert_eq!(1, spec.devices.borrow().len());
        assert_eq!("01_7E31", device_spec.device_id);

        let device_spec = spec.get_device_spec(0x00, 0x7E31, 0x0010);

        assert_eq!(2, spec.devices.borrow().len());
        assert_eq!("00_7E31", device_spec.device_id);
        assert_eq!(0x00, device_spec.channel);
        assert_eq!(0x7E31, device_spec.self_address);
        assert_eq!(None, device_spec.peer_address);
        assert_eq!("DeltaSol MX [WMZ #1]", device_spec.name);

        let device_spec = spec.get_device_spec(0x00, 0x7E11, 0x0010);

        assert_eq!(3, spec.devices.borrow().len());
        assert_eq!("00_7E11", device_spec.device_id);
        assert_eq!(0x00, device_spec.channel);
        assert_eq!(0x7E11, device_spec.self_address);
        assert_eq!(None, device_spec.peer_address);
        assert_eq!("Unknown device 0x7E11", device_spec.name);
    }

    #[test]
    fn test_get_packet_spec() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7E31, 0x0100);

        assert_eq!(1, spec.packets.borrow().len());
        assert_eq!("01_0010_7E31_10_0100", packet_spec.packet_id);
        assert_eq!(0x01, packet_spec.channel);
        assert_eq!(0x0010, packet_spec.destination_address);
        assert_eq!(0x7E31, packet_spec.source_address);
        assert_eq!(0x0100, packet_spec.command);
        assert_eq!("VBus 1: DFA", packet_spec.destination_device.name);
        assert_eq!(
            "VBus 1: DeltaSol MX [WMZ #1]",
            packet_spec.source_device.name
        );
        assert_eq!("VBus 1: DeltaSol MX [WMZ #1]", packet_spec.name);
        assert_eq!(8, packet_spec.fields.len());

        let field_spec = &packet_spec.fields[0];
        assert_eq!("000_4_0", field_spec.field_id);
        assert_eq!("01_0010_7E31_10_0100_000_4_0", field_spec.packet_field_id);
        assert_eq!("Heat quantity", field_spec.name);
        assert_eq!(18, field_spec.unit_id.0);
        assert_eq!(UnitFamily::Energy, field_spec.unit_family);
        assert_eq!("WattHours", field_spec.unit_code);
        assert_eq!(" Wh", field_spec.unit_text);
        assert_eq!(0, field_spec.precision);
        assert_eq!(Type::Number, field_spec.typ);
        assert_eq!(8, field_spec.parts.len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7E31, 0x0100);

        assert_eq!(1, spec.packets.borrow().len());
        assert_eq!("01_0010_7E31_10_0100", packet_spec.packet_id);

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E31, 0x0100);

        assert_eq!(2, spec.packets.borrow().len());
        assert_eq!("00_0010_7E31_10_0100", packet_spec.packet_id);
        assert_eq!(0x00, packet_spec.channel);
        assert_eq!(0x0010, packet_spec.destination_address);
        assert_eq!(0x7E31, packet_spec.source_address);
        assert_eq!(0x0100, packet_spec.command);
        assert_eq!("DFA", packet_spec.destination_device.name);
        assert_eq!("DeltaSol MX [WMZ #1]", packet_spec.source_device.name);
        assert_eq!("DeltaSol MX [WMZ #1]", packet_spec.name);
        assert_eq!(8, packet_spec.fields.len());

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);

        assert_eq!(3, spec.packets.borrow().len());
        assert_eq!("00_0010_7E11_10_0100", packet_spec.packet_id);
        assert_eq!(0x00, packet_spec.channel);
        assert_eq!(0x0010, packet_spec.destination_address);
        assert_eq!(0x7E11, packet_spec.source_address);
        assert_eq!(0x0100, packet_spec.command);
        assert_eq!("DFA", packet_spec.destination_device.name);
        assert_eq!("Unknown device 0x7E11", packet_spec.source_device.name);
        assert_eq!("Unknown device 0x7E11", packet_spec.name);
        assert_eq!(0, packet_spec.fields.len());
    }

    #[test]
    fn test_get_packet_spec_by_id() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        let packet_spec = spec.get_packet_spec_by_id(PacketId(0x00, 0x0010, 0x7E11, 0x0100));

        assert_eq!("00_0010_7E11_10_0100", packet_spec.packet_id);
    }

    #[test]
    fn test_get_field_spec() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7E31, 0x0100);

        let field_spec = packet_spec.get_field_spec("000_4_0").unwrap();
        assert_eq!("000_4_0", field_spec.field_id);
        assert_eq!("01_0010_7E31_10_0100_000_4_0", field_spec.packet_field_id);
        assert_eq!("Heat quantity", field_spec.name);
        assert_eq!(18, field_spec.unit_id.0);
        assert_eq!(UnitFamily::Energy, field_spec.unit_family);
        assert_eq!("WattHours", field_spec.unit_code);
        assert_eq!(" Wh", field_spec.unit_text);
        assert_eq!(0, field_spec.precision);
        assert_eq!(Type::Number, field_spec.typ);
        assert_eq!(8, field_spec.parts.len());

        assert_eq!(None, packet_spec.get_field_spec("000_2_0"));
    }

    #[test]
    fn test_raw_value_i64() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7F61, 0x0100);

        let buf = &[
            0x78, 0x56, 0x34, 0x12, 0xB8, 0x22, 0x00, 0x00, 0x48, 0xDD, 0xFF, 0xFF,
        ];

        assert_eq!(
            Some(0x12345678),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_i64(buf)
        );
        assert_eq!(
            Some(8888),
            packet_spec
                .get_field_spec("004_4_0")
                .unwrap()
                .raw_value_i64(buf)
        );
        assert_eq!(
            Some(-8888),
            packet_spec
                .get_field_spec("008_4_0")
                .unwrap()
                .raw_value_i64(buf)
        );
        assert_eq!(
            Some(0x345678),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_i64(&buf[0..3])
        );
        assert_eq!(
            Some(0x5678),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_i64(&buf[0..2])
        );
        assert_eq!(
            Some(0x78),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_i64(&buf[0..1])
        );
        assert_eq!(
            None,
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_i64(&buf[0..0])
        );

        let pfs = PacketFieldSpec {
            field_id: "FieldID".into(),
            packet_field_id: "PacketFieldId".into(),
            name: "Name".into(),
            unit_id: UnitId(0),
            unit_family: UnitFamily::None,
            unit_code: "UnitCode".into(),
            unit_text: "UnitText".into(),
            precision: 0,
            typ: Type::Number,
            parts: vec![PacketTemplateFieldPart {
                offset: 9,
                bit_pos: 3,
                mask: 0x38,
                is_signed: false,
                factor: 1,
            }],
            language: Language::En,
        };

        assert_eq!(Some(3), pfs.raw_value_i64(&buf[..]));
    }

    #[test]
    fn test_raw_value_f64() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7F61, 0x0100);

        let buf = &[
            0x78, 0x56, 0x34, 0x12, 0xB8, 0x22, 0x00, 0x00, 0x48, 0xDD, 0xFF, 0xFF,
        ];

        assert_eq!(
            Some(0x12345678 as f64),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_f64(buf)
        );
        assert_eq!(
            Some(888.8000000000001),
            packet_spec
                .get_field_spec("004_4_0")
                .unwrap()
                .raw_value_f64(buf)
        );
        assert_eq!(
            Some(-888.8000000000001),
            packet_spec
                .get_field_spec("008_4_0")
                .unwrap()
                .raw_value_f64(buf)
        );
        assert_eq!(
            Some(0x345678 as f64),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_f64(&buf[0..3])
        );
        assert_eq!(
            Some(0x5678 as f64),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_f64(&buf[0..2])
        );
        assert_eq!(
            Some(0x78 as f64),
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_f64(&buf[0..1])
        );
        assert_eq!(
            None,
            packet_spec
                .get_field_spec("000_4_0")
                .unwrap()
                .raw_value_f64(&buf[0..0])
        );
    }

    #[test]
    fn test_fmt_raw_value() {
        let fake_field_spec = |precision, typ, unit_text: &str| PacketFieldSpec {
            field_id: "".to_string(),
            packet_field_id: "".to_string(),
            name: "".to_string(),
            unit_id: UnitId(0),
            unit_family: UnitFamily::None,
            unit_code: "unit code".to_string(),
            unit_text: unit_text.to_string(),
            precision,
            typ,
            parts: Vec::new(),
            language: Language::En,
        };

        let fmt_raw_value = |field_spec: &PacketFieldSpec, raw_value, append_unit| {
            let test_value = field_spec.fmt_raw_value(Some(raw_value), append_unit);
            format!("{}", test_value)
        };

        let field_spec = fake_field_spec(0, Type::Number, "don't append unit");
        assert_eq!("12346", fmt_raw_value(&field_spec, 12346, false));

        let field_spec = fake_field_spec(0, Type::Number, " unit");
        assert_eq!("12346 unit", fmt_raw_value(&field_spec, 12346, true));

        let field_spec = fake_field_spec(1, Type::Number, "don't append unit");
        assert_eq!("12345.7", fmt_raw_value(&field_spec, 123457, false));

        let field_spec = fake_field_spec(2, Type::Number, "don't append unit");
        assert_eq!("12345.68", fmt_raw_value(&field_spec, 1234568, false));

        let field_spec = fake_field_spec(3, Type::Number, "don't append unit");
        assert_eq!("12345.679", fmt_raw_value(&field_spec, 12345679, false));

        let field_spec = fake_field_spec(4, Type::Number, "don't append unit");
        assert_eq!("12345.6789", fmt_raw_value(&field_spec, 123456789, false));

        let field_spec = fake_field_spec(4, Type::Number, "don't append unit");
        assert_eq!("12345.0009", fmt_raw_value(&field_spec, 123450009, false));

        let field_spec = fake_field_spec(5, Type::Number, "don't append unit");
        assert_eq!("12345.00098", fmt_raw_value(&field_spec, 1234500098, false));

        let field_spec = fake_field_spec(6, Type::Number, "don't append unit");
        assert_eq!(
            "12345.000987",
            fmt_raw_value(&field_spec, 12345000987, false)
        );

        let field_spec = fake_field_spec(7, Type::Number, "don't append unit");
        assert_eq!(
            "12345.0009876",
            fmt_raw_value(&field_spec, 123450009876, false)
        );

        let field_spec = fake_field_spec(8, Type::Number, "don't append unit");
        assert_eq!(
            "12345.00098765",
            fmt_raw_value(&field_spec, 1234500098765, false)
        );

        let field_spec = fake_field_spec(9, Type::Number, "don't append unit");
        assert_eq!(
            "12345.000987654",
            fmt_raw_value(&field_spec, 12345000987654, false)
        );

        let field_spec = fake_field_spec(10, Type::Number, "don't append unit");
        assert_eq!(
            "1.2345678900",
            fmt_raw_value(&field_spec, 12345678900, false)
        );

        let field_spec = fake_field_spec(10, Type::Time, "don't append unit");
        assert_eq!("12:01", fmt_raw_value(&field_spec, 721, true));

        let field_spec = fake_field_spec(10, Type::WeekTime, "don't append unit");
        assert_eq!("Th,12:01", fmt_raw_value(&field_spec, 3 * 1440 + 721, true));

        let field_spec = fake_field_spec(10, Type::DateTime, "don't append unit");
        assert_eq!(
            "22/12/2013 15:17:42",
            fmt_raw_value(&field_spec, 409418262, true)
        );

        let formatter = field_spec.fmt_raw_value(None, true);
        assert_eq!("", format!("{}", formatter));
    }

    #[test]
    fn test_fields_in_data_set() {
        let mut rr = RecordingReader::new(RECORDING_2);

        let data_set = rr.read_data_set().unwrap().unwrap();

        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        let fields = spec.fields_in_data_set(&data_set).collect::<Vec<_>>();

        assert_eq!(8, fields.len());

        let field = &fields[0];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(0, field.field_index());
        assert_eq!("000_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 Wh", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[1];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(1, field.field_index());
        assert_eq!("008_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 Wh", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[2];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(2, field.field_index());
        assert_eq!("012_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 Wh", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[3];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(3, field.field_index());
        assert_eq!("020_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 Wh", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[4];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(4, field.field_index());
        assert_eq!("016_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 l", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[5];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(5, field.field_index());
        assert_eq!("024_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 l", format!("{}", field.fmt_raw_value(true)));

        let field = &fields[6];
        assert_eq!(1, field.data_index());
        assert_eq!(&data_set.as_data_slice()[1], field.data());
        assert_eq!("00_0010_7E31_10_0100", field.packet_spec().packet_id);
        assert_eq!(6, field.field_index());
        assert_eq!("028_4_0", field.field_spec().field_id);
        assert_eq!(Some(0f64), field.raw_value_f64());
        assert_eq!("0", format!("{}", field.fmt_raw_value(false)));
        assert_eq!("0 l", format!("{}", field.fmt_raw_value(true)));
    }

    #[test]
    fn test_unit_by_unit_code() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let unit = spec
            .unit_by_unit_code("DegreesCelsius")
            .expect("Unit should exist");

        assert_eq!(UnitId(62), unit.unit_id);

        assert!(spec.unit_by_unit_code("Unknown").is_none());
    }

    #[test]
    fn test_convert_value() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let src_unit = spec
            .unit_by_unit_code("DegreesCelsius")
            .expect("Source unit should exist");

        let dst_unit = spec
            .unit_by_unit_code("DegreesFahrenheit")
            .expect("Destination unit should exist");

        let converted_value = spec
            .convert_value(20.0, src_unit, dst_unit)
            .expect("Conversion should work");

        assert_eq!(68.0, converted_value);
    }

    #[test]
    fn test_packet_spec_get_field_spec_position() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);

        assert_eq!(Some(1), packet_spec.get_field_spec_position("002_2_0"));
        assert_eq!(None, packet_spec.get_field_spec_position("002_1_0"));
    }

    #[test]
    fn test_packet_spec_get_field_spec_by_position() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);

        assert_eq!(
            "002_2_0",
            packet_spec.get_field_spec_by_position(1).field_id
        );
    }

    #[test]
    fn test_data_set_packet_field_new() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);

        let mut data_set = DataSet::new();

        data_set.add_data(Data::Packet(Packet {
            header: Header {
                timestamp: utc_timestamp(1485688933),
                channel: 0,
                destination_address: 0x0010,
                source_address: 0x7E11,
                protocol_version: 0x10,
            },
            command: 0x0100,
            frame_count: 10,
            frame_data: [0xA5; 508],
        }));

        let dspf = DataSetPacketField::new(&data_set, 0, packet_spec.clone(), 0, Some(1234));

        assert_eq!(1, dspf.data_set.len());
        assert_eq!(0, dspf.data_index);
        assert_eq!("00_0010_7E11_10_0100", &dspf.packet_spec.packet_id);
        assert_eq!(0, dspf.field_index);
        assert_eq!(Some(1234), dspf.raw_value);
    }

    #[test]
    fn test_data_set_packet_field_accessors() {
        let spec = Specification::from_file(SpecificationFile::new_default(), Language::En);

        let packet_spec = spec.get_packet_spec(0x00, 0x0010, 0x7E11, 0x0100);

        let mut data_set = DataSet::new();

        data_set.add_data(Data::Packet(Packet {
            header: Header {
                timestamp: utc_timestamp(1485688933),
                channel: 0,
                destination_address: 0x0010,
                source_address: 0x7E11,
                protocol_version: 0x10,
            },
            command: 0x0100,
            frame_count: 10,
            frame_data: [0xA5; 508],
        }));

        let dspf = DataSetPacketField::new(&data_set, 0, packet_spec.clone(), 0, Some(1234));

        let data_set = dspf.data_set();

        assert_eq!(1, data_set.len());
        assert_eq!("00_0010_7E11_10_0100", data_set[0].id_string());

        assert_eq!(0, dspf.data_index());
        assert_eq!("00_0010_7E11_10_0100", dspf.data().id_string());
        assert_eq!("00_0010_7E11_10_0100", dspf.packet_spec().packet_id);
        assert_eq!(0, dspf.field_index());
        assert_eq!(
            "00_0010_7E11_10_0100_000_2_0",
            dspf.field_spec().packet_field_id
        );
        assert_eq!(PacketId(0x00, 0x0010, 0x7E11, 0x0100), dspf.packet_id());
        assert_eq!("000_2_0", dspf.field_id());
        assert_eq!(
            PacketFieldId(PacketId(0x00, 0x0010, 0x7E11, 0x0100), "000_2_0"),
            dspf.packet_field_id()
        );
        assert_eq!(&Some(1234), dspf.raw_value_i64());
        assert_eq!(Some(123.4), dspf.raw_value_f64());
    }
}
