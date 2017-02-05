use std::cell::RefCell;
use std::clone::Clone;
use std::fmt;
use std::sync::Arc;

use chrono::{TimeZone, UTC};

use specification_file::{SpecificationFile, Language, UnitFamily, UnitId, Type, PacketTemplateFieldPart};


/// Contains information about a VBus device.
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
    pub destination_device: Arc<DeviceSpec>,

    /// The `DeviceSpec` containing information about the source VBus device.
    pub source_device: Arc<DeviceSpec>,

    /// The name of the packet, containing channel, source and optionally destination names.
    pub name: String,

    /// The fields contained in the frame payload of the VBus packet.
    pub fields: Vec<PacketFieldSpec>,
}


/// Contains information about a VBus packet field.
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
}


/// A helper type for formatting raw values.
#[derive(Debug)]
pub struct PacketFieldFormatter<'a> {
    typ: Type,
    precision: usize,
    raw_value: f64,
    unit_text: &'a str,
}


/// The `Specification` type contains information about known devices and packets.
#[derive(Debug)]
pub struct Specification {
    file: SpecificationFile,
    language: Language,
    devices: RefCell<Vec<Arc<DeviceSpec>>>,
    packets: RefCell<Vec<Arc<PacketSpec>>>,
}


fn get_cached_device_spec(devices: &Vec<Arc<DeviceSpec>>, channel: u8, self_address: u16, peer_address: u16) -> Option<Arc<DeviceSpec>> {
    let result = devices.iter().find(|&device| {
        if device.channel != channel {
            false
        } else if device.self_address != self_address {
            false
        } else if device.peer_address.is_some() && device.peer_address.unwrap() != peer_address {
            false
        } else {
            true
        }
    });

    match result {
        Some(ref device) => Some((*device).clone()),
        None => None,
    }
}


fn get_or_create_cached_device_spec(devices: &mut Vec<Arc<DeviceSpec>>, channel: u8, self_address: u16, peer_address: u16, file: &SpecificationFile, language: &Language) -> Arc<DeviceSpec> {
    if let Some(device) = get_cached_device_spec(devices, channel, self_address, peer_address) {
        return device;
    }

    let device_template = match file.find_device_template(self_address, peer_address) {
        Some(device_template) => Some(device_template.clone()),
        None => None,
    };

    let peer_address_option = match device_template {
        None => None,
        Some(device_template) => if device_template.peer_mask == 0 {
            None
        } else {
            Some(peer_address)
        }
    };

    let device_id = match peer_address_option {
        None => format!("{:02X}_{:04X}", channel, self_address),
        Some(peer_address) => format!("{:02X}_{:04X}_{:04X}", channel, self_address, peer_address),
    };

    let name = match device_template {
        None => {
            match language {
                &Language::En => format!("Unknown device 0x{:04X}", self_address),
                &Language::De => format!("Unbekanntes Gerät 0x{:04X}", self_address),
                &Language::Fr => format!("Unknown device 0x{:04X}", self_address),  // FIXME(daniel): missing translation
            }
        },
        Some(device_template) => {
            let format = file.localized_text_by_index(&device_template.name_localized_text_index, language);
            format.to_string()  // FIXME(daniel): replace module nr placeholder
        }
    };

    let name = match channel {
        0 => name,
        _ => format!("VBus {}: {}", channel, name),
    };

    let device = DeviceSpec {
        device_id: device_id,
        channel: channel,
        self_address: self_address,
        peer_address: peer_address_option,
        name: name,
    };

    devices.push(Arc::new(device));

    get_cached_device_spec(devices, channel, self_address, peer_address).unwrap()
}


fn get_cached_packet_spec(packets: &Vec<Arc<PacketSpec>>, channel: u8, destination_address: u16, source_address: u16, command: u16) -> Option<Arc<PacketSpec>> {
    let result = packets.iter().find(|&packet| {
        if packet.channel != channel {
            false
        } else if packet.destination_address != destination_address {
            false
        } else if packet.source_address != source_address {
            false
        } else if packet.command != command {
            false
        } else {
            true
        }
    });

    match result {
        Some(ref packet) => Some((*packet).clone()),
        None => None,
    }
}


fn get_or_create_cached_packet_spec(packets: &mut Vec<Arc<PacketSpec>>, channel: u8, destination_address: u16, source_address: u16, command: u16, devices: &mut Vec<Arc<DeviceSpec>>, file: &SpecificationFile, language: &Language) -> Arc<PacketSpec> {
    if let Some(packet) = get_cached_packet_spec(packets, channel, destination_address, source_address, command) {
        return packet;
    }

    let destination_device = get_or_create_cached_device_spec(devices, channel, destination_address, source_address, file, language);
    let source_device = get_or_create_cached_device_spec(devices, channel, source_address, destination_address, file, language);

    let packet_id = format!("{:02X}_{:04X}_{:04X}_10_{:04X}", channel, destination_address, source_address, command);

    let packet_name = match destination_address {
        0x0010 => source_device.name.clone(),
        _ => format!("{} => {}", source_device.name, destination_device.name),
    };

    let fields = match file.find_packet_template(destination_address, source_address, command) {
        None => Vec::new(),
        Some(packet_template) => {
            packet_template.fields.iter().map(|field| {
                let field_id = file.text_by_index(&field.id_text_index).to_string();

                let packet_field_id = format!("{}_{}", packet_id, field_id);

                let field_name = file.localized_text_by_index(&field.name_localized_text_index, &language).to_string();

                let unit = file.unit_by_id(&field.unit_id);

                let unit_family = file.unit_family_by_id(&unit.unit_family_id);
                let unit_code = file.text_by_index(&unit.unit_code_text_index).to_string();
                let unit_text = file.text_by_index(&unit.unit_text_text_index).to_string();

                let typ = file.type_by_id(&field.type_id);

                PacketFieldSpec {
                    field_id: field_id,
                    packet_field_id: packet_field_id,
                    name: field_name,
                    unit_id: field.unit_id.clone(),
                    unit_family: unit_family,
                    unit_code: unit_code,
                    unit_text: unit_text,
                    precision: field.precision,
                    typ: typ,
                    parts: field.parts.clone(),
                }
            }).collect()
        },
    };

    let packet = PacketSpec {
        packet_id: packet_id,
        channel: channel,
        destination_address: destination_address,
        source_address: source_address,
        command: command,
        destination_device: destination_device,
        source_device: source_device,
        name: packet_name,
        fields: fields,
    };

    packets.push(Arc::new(packet));

    get_cached_packet_spec(packets, channel, destination_address, source_address, command).unwrap()
}


/// Get the "power of 10" value for common "n"s and calculate it otherwise.
pub fn get_power_of_ten(n: i32) -> f64 {
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
        _ => 10.0f64.powf(n as f64),
    }
}


impl Specification {

    /// Construct a `Specification` from a `SpecificationFile` and a `Language`.
    pub fn from_file(file: SpecificationFile, language: Language) -> Specification {
        let devices = RefCell::new(Vec::new());
        let packets = RefCell::new(Vec::new());

        Specification {
            file: file,
            language: language,
            devices: devices,
            packets: packets,
        }
    }

    /// Get a `DeviceSpec`.
    pub fn get_device_spec(&self, channel: u8, self_address: u16, peer_address: u16) -> Arc<DeviceSpec> {
        let mut devices = self.devices.borrow_mut();
        get_or_create_cached_device_spec(&mut devices, channel, self_address, peer_address, &self.file, &self.language)
    }

    /// Get a `PacketSpec`.
    pub fn get_packet_spec(&self, channel: u8, destination_address: u16, source_address: u16, command: u16) -> Arc<PacketSpec> {
        let mut devices = self.devices.borrow_mut();
        let mut packets = self.packets.borrow_mut();
        get_or_create_cached_packet_spec(&mut packets, channel, destination_address, source_address, command, &mut devices, &self.file, &self.language)
    }

}


impl PacketSpec {

    /// Get a `PacketFieldSpec` by its field ID.
    pub fn get_field_spec(&self, id: &str) -> Option<&PacketFieldSpec> {
        self.fields.iter().find(|field_spec| field_spec.field_id == id)
    }

}


impl PacketFieldSpec {

    /// Construct an `i64` raw value from a slice of bytes.
    pub fn get_raw_value_i64(&self, buf: &[u8]) -> Option<i64> {
        let length = buf.len();

        let mut valid = false;
        let mut raw_value = 0;

        for part in &self.parts {
            let offset = part.offset as usize;

            if offset < length {
                let mut part_value = if part.is_signed {
                    (buf [offset] as i8) as i64
                } else {
                    buf [offset] as i64
                };
                if part.mask != 0xFF {
                    part_value &= part.mask as i64;
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
    pub fn get_raw_value_f64(&self, buf: &[u8]) -> Option<f64> {
        match self.get_raw_value_i64(buf) {
            Some(raw_value) => Some(raw_value as f64 * get_power_of_ten(- self.precision)),
            None => None
        }
    }

    /// Format a raw value into its textual representation.
    pub fn fmt_raw_value<'a>(&'a self, raw_value: f64, append_unit: bool) -> PacketFieldFormatter<'a> {
        let unit_text = if append_unit {
            &self.unit_text
        } else {
            ""
        };
        PacketFieldFormatter {
            typ: self.typ.clone(),
            precision: self.precision as usize,
            raw_value: raw_value,
            unit_text: unit_text,
        }
    }

}


impl<'a> fmt::Display for PacketFieldFormatter<'a> {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            Type::Number => {
                write!(f, "{:.*}{}", self.precision, self.raw_value, self.unit_text)
            }
            Type::Time => {
                let raw_value = self.raw_value.round() as i64;
                let hours = raw_value / 60;
                let minutes = raw_value % 60;
                write!(f, "{:02}:{:02}", hours, minutes)
            }
            Type::WeekTime => {
                let raw_value = self.raw_value.round() as i64;
                let timestamp = UTC.timestamp(raw_value * 60 + 4 * 86400, 0);
                write!(f, "{}", timestamp.format("%a,%H:%M"))
            }
            Type::DateTime => {
                let raw_value = self.raw_value.round() as i64;
                let timestamp = UTC.timestamp(raw_value + 978307200, 0);
                write!(f, "{}", timestamp.format("%Y-%m-%d %H:%M:%S"))
            }
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    use test_data::{SPEC_FILE_1};

    #[test]
    fn test_get_power_of_10() {
        for n in -20..20 {
            assert_eq!(10.0f64.powf(n as f64), get_power_of_ten(n));
        }
    }

    #[test]
    fn test_from_file() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.devices.borrow().len());
        assert_eq!(0, spec.packets.borrow().len());
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
        assert_eq!("VBus 1: DeltaSol MX [WMZ #1]", packet_spec.source_device.name);
        assert_eq!("VBus 1: DeltaSol MX [WMZ #1]", packet_spec.name);
        assert_eq!(8, packet_spec.fields.len());

        let field_spec = &packet_spec.fields [0];
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
    fn test_get_raw_value_i64() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7F61, 0x0100);

        let buf = &[
            0x78, 0x56, 0x34, 0x12,
            0xB8, 0x22, 0x00, 0x00,
            0x48, 0xDD, 0xFF, 0xFF,
        ];

        assert_eq!(Some(0x12345678), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_i64(buf));
        assert_eq!(Some(8888), packet_spec.get_field_spec("004_4_0").unwrap().get_raw_value_i64(buf));
        assert_eq!(Some(-8888), packet_spec.get_field_spec("008_4_0").unwrap().get_raw_value_i64(buf));
        assert_eq!(Some(0x345678), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_i64(&buf [0..3]));
        assert_eq!(Some(0x5678), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_i64(&buf [0..2]));
        assert_eq!(Some(0x78), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_i64(&buf [0..1]));
        assert_eq!(None, packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_i64(&buf [0..0]));
    }

    #[test]
    fn test_get_raw_value_f64() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        let spec = Specification::from_file(spec_file, Language::En);

        assert_eq!(0, spec.packets.borrow().len());

        let packet_spec = spec.get_packet_spec(0x01, 0x0010, 0x7F61, 0x0100);

        let buf = &[
            0x78, 0x56, 0x34, 0x12,
            0xB8, 0x22, 0x00, 0x00,
            0x48, 0xDD, 0xFF, 0xFF,
        ];

        assert_eq!(Some(0x12345678 as f64), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_f64(buf));
        assert_eq!(Some(888.8000000000001), packet_spec.get_field_spec("004_4_0").unwrap().get_raw_value_f64(buf));
        assert_eq!(Some(-888.8000000000001), packet_spec.get_field_spec("008_4_0").unwrap().get_raw_value_f64(buf));
        assert_eq!(Some(0x345678 as f64), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_f64(&buf [0..3]));
        assert_eq!(Some(0x5678 as f64), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_f64(&buf [0..2]));
        assert_eq!(Some(0x78 as f64), packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_f64(&buf [0..1]));
        assert_eq!(None, packet_spec.get_field_spec("000_4_0").unwrap().get_raw_value_f64(&buf [0..0]));
    }

    #[test]
    fn test_fmt_raw_value() {
        let fake_field_spec = |precision, typ, unit_text: &str| {
            PacketFieldSpec {
                field_id: "".to_string(),
                packet_field_id: "".to_string(),
                name: "".to_string(),
                unit_id: UnitId(0),
                unit_family: UnitFamily::None,
                unit_code: "unit code".to_string(),
                unit_text: unit_text.to_string(),
                precision: precision,
                typ: typ,
                parts: Vec::new(),
            }
        };

        let fmt_raw_value = |field_spec: &PacketFieldSpec, raw_value, append_unit| {
            let test_value = field_spec.fmt_raw_value(raw_value, append_unit);
            format!("{}", test_value)
        };

        let field_spec = fake_field_spec(0, Type::Number, "don't append unit");
        assert_eq!("12346", fmt_raw_value(&field_spec, 12345.6789, false));

        let field_spec = fake_field_spec(0, Type::Number, " unit");
        assert_eq!("12346 unit", fmt_raw_value(&field_spec, 12345.6789, true));

        let field_spec = fake_field_spec(1, Type::Number, "don't append unit");
        assert_eq!("12345.7", fmt_raw_value(&field_spec, 12345.6789, false));

        let field_spec = fake_field_spec(2, Type::Number, "don't append unit");
        assert_eq!("12345.68", fmt_raw_value(&field_spec, 12345.6789, false));

        let field_spec = fake_field_spec(3, Type::Number, "don't append unit");
        assert_eq!("12345.679", fmt_raw_value(&field_spec, 12345.6789, false));

        let field_spec = fake_field_spec(4, Type::Number, "don't append unit");
        assert_eq!("12345.6789", fmt_raw_value(&field_spec, 12345.6789, false));

        let field_spec = fake_field_spec(10, Type::Number, "don't append unit");
        assert_eq!("1.2345678900", fmt_raw_value(&field_spec, 1.23456789, false));

        let field_spec = fake_field_spec(10, Type::Time, "don't append unit");
        assert_eq!("12:01", fmt_raw_value(&field_spec, 721.0, true));

        let field_spec = fake_field_spec(10, Type::WeekTime, "don't append unit");
        assert_eq!("Thu,12:01", fmt_raw_value(&field_spec, 3.0 * 1440.0 + 721.0, true));

        let field_spec = fake_field_spec(10, Type::DateTime, "don't append unit");
        assert_eq!("2013-12-22 15:17:42", fmt_raw_value(&field_spec, 409418262.0, true));
    }
}