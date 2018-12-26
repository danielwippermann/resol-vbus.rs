//! A module that parses the contents of a VBus Specification File Version 1 (VSF1).
//!
//! The VBus Specification File format is used to provide information about VBus Protocol Version
//! 1.x `Packet`s and their frame data payload.
//!
//! See the [RESOL VBus Specification File Format v1](http://danielwippermann.github.io/resol-vbus/vbus-specification-file-format-v1.html)
//! for details.
use byteorder::{ByteOrder, LittleEndian};

use error::{Error, Result};
use utils::calc_crc16;

/// A list of errors that can occur if the VSF1 data cannot be parsed.
#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
    /// The data is too small for a valid FILEHEADER.
    InvalidFileHeader,

    /// The data length does not match the "TotalLength" field of the FILEHEADER.
    InvalidFileHeaderTotalLength,
    /// The data does not match the "ChecksumA" field of the FILEHEADER.
    InvalidFileHeaderChecksumA,
    /// The data does not match the "ChecksumB" field of the FILEHEADER.
    InvalidFileHeaderChecksumB,
    /// The "DataVersion" field of the FILEHEADER is not supported.
    InvalidFileHeaderDataVersion,
    /// The "SpecificationOffset" of the FILEHEADER is out-of-bounds.
    InvalidFileHeaderSpecificationOffset,

    /// The "Text{Count,TableOffset}" fields of the SPECIFICATION block are out-of-bounds.
    InvalidSpecificationTextTable,
    /// The "LocalizedText{Count,TableOffset}" fields of the SPECIFICATION block are out-of-bounds.
    InvalidSpecificationLocalizedTextTable,
    /// The "Unit{Count,TableOffset}" fields of the SPECIFICATION block are out-of-bounds.
    InvalidSpecificationUnitTable,
    /// The "DeviceTemplate{Count,TableOffset}" fields of the SPECIFICATION block are out-of-bounds.
    InvalidSpecificationDeviceTemplateTable,
    /// The "PacketTemplate{Count,TableOffset}" fields of the SPECIFICATION block are out-of-bounds.
    InvalidSpecificationPacketTemplateTable,

    /// The "StringOffset" field of a TEXT block is out-of-bounds.
    InvalidTextStringOffset,
    /// The contents of a TEXT is out-of-bounds.
    InvalidTextContent,

    /// The "TextIndexEN" field of a LOCALIZEDTEXT block is out-of-bounds.
    InvalidLocalizedTextTextIndexEn,
    /// The "TextIndexDE" field of a LOCALIZEDTEXT block is out-of-bounds.
    InvalidLocalizedTextTextIndexDe,
    /// The "TextIndexFR" field of a LOCALIZEDTEXT block is out-of-bounds.
    InvalidLocalizedTextTextIndexFr,

    /// The "UnitFamilyId" field of a UNIT block is out-of-bounds.
    InvalidUnitUnitFamilyId,
    /// The "UnitCodeTextIndex" field of a UNIT block is out-of-bounds.
    InvalidUnitUnitCodeTextIndex,
    /// The "UnitTextTextIndex" field of a UNIT block is out-of-bounds.
    InvalidUnitUnitTextTextIndex,

    /// The "NameLocalizedTextIndex" field of a DEVICETEMPLATE block is out-of-bounds.
    InvalidDeviceTemplateNameLocalizedTextIndex,

    /// The "Field{Count,TableOffset}" fields of a PACKETTEMPLATE block are out-of-bounds.
    InvalidPacketTemplateFieldTable,

    /// The "IdTextIndex" of a PACKETTEMPLATEFIELD block is out-of-bounds.
    InvalidPacketTemplateFieldIdTextIndex,
    /// The "NameLocalizedTextIndex" of a PACKETTEMPLATEFIELD block is out-of-bounds.
    InvalidPacketTemplateFieldNameLocalizedTextIndex,
    /// The "UnitId" of a PACKETTEMPLATEFIELD block is out-of-bounds.
    InvalidPacketTemplateFieldUnitId,
    /// The "TypeId" of a PACKETTEMPLATEFIELD block is out-of-bounds.
    InvalidPacketTemplateFieldTypeId,
    /// The "Part{Count,TableOffset}" of a PACKETTEMPLATEFIELD is out-of-bounds.
    InvalidPacketTemplateFieldPartTable,
}

fn err<T>(kind: ErrorKind) -> Result<T> {
    Err(Error::new(format!("Unable to parse VSF: {:?}", kind)))
}

fn check_offset(buf: &[u8], offset: usize, length: usize, count: usize) -> bool {
    let end_offset = offset + length * count;
    end_offset <= buf.len()
}

fn slice_entry(buf: &[u8], offset: usize, length: usize) -> &[u8] {
    &buf[offset..(offset + length)]
}

fn slice_table_entry(buf: &[u8], offset: usize, length: usize, index: usize) -> &[u8] {
    let table_entry_offset = offset + (index * length);
    slice_entry(buf, table_entry_offset, length)
}

/// Languages supported by VSF1 specification.
///
/// # Examples
///
/// ```rust
/// use resol_vbus::{Specification, SpecificationFile, Language};
///
/// let get_loc_text = |language| {
///     // Create a new `Specification` for the provided language
///     let spec = Specification::from_file(SpecificationFile::new_default(), language);
///
///     // Get the `PacketSpec` for the standard info packet of the "DeltaSol MX"
///     let packet_spec = spec.get_packet_spec(0x11, 0x0010, 0x7E11, 0x0100);
///
///     // Get the first `PacketFieldSpec`'s name.
///     packet_spec.get_field_spec("000_2_0").map(|field_spec| field_spec.name.clone()).unwrap()
/// };
///
/// assert_eq!("Temperature sensor 1", get_loc_text(Language::En));
/// assert_eq!("Temperatur Sensor 1", get_loc_text(Language::De));
/// assert_eq!("Température sonde 1", get_loc_text(Language::Fr));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    /// English language
    En,

    /// German language
    De,

    /// French language
    Fr,
}

/// A numeric reference to a `Text`.
#[derive(Clone, Copy, Debug)]
pub struct TextIndex(i32);

/// Combines three `TextIndex` values for each of the supported languages to form a localized text.
#[derive(Debug)]
pub struct LocalizedText {
    /// A `TextIndex` to the english text.
    pub text_index_en: TextIndex,

    /// A `TextIndex` to the german text.
    pub text_index_de: TextIndex,

    /// A `TextIndex` to the french text.
    pub text_index_fr: TextIndex,
}

/// A numeric reference to a `LocalizedText` instance.
#[derive(Clone, Copy, Debug)]
pub struct LocalizedTextIndex(i32);

/// A numeric reference to an `UnitFamily` instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitFamilyId(pub i32);

/// One of the unit families supported by the VSF1 specification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitFamily {
    /// Not associated with a unit family.
    None,

    /// Temperature
    Temperature,

    /// Energy
    Energy,

    /// Volume flow
    VolumeFlow,

    /// Pressure
    Pressure,

    /// Volume
    Volume,

    /// Time
    Time,

    /// Power
    Power,
}

/// A numeric reference to an `Unit` instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitId(pub i32);

/// A physical unit.
#[derive(Debug)]
pub struct Unit {
    /// The numeric ID of the `Unit`.
    pub unit_id: UnitId,

    /// The numeric ID of the `UnitFamily`.
    pub unit_family_id: UnitFamilyId,

    /// The `TextIndex` of the unit's machine-readable name.
    pub unit_code_text_index: TextIndex,

    /// The `TextIndex` of the unit's human-readable name.
    pub unit_text_text_index: TextIndex,
}

/// Contains information about a VBus device.
#[derive(Debug)]
pub struct DeviceTemplate {
    /// The VBus address of the device itself.
    pub self_address: u16,

    /// The mask applied to the VBus address of the device itself.
    pub self_mask: u16,

    /// The VBus address of a potential peer device.
    pub peer_address: u16,

    /// The mask applied to the VBus address of the potential peer device.
    pub peer_mask: u16,

    /// The `LocalizedTextIndex` of the device's name.
    pub name_localized_text_index: LocalizedTextIndex,
}

/// Contains information about a VBus packet.
#[derive(Clone, Debug)]
pub struct PacketTemplate {
    /// The VBus address of the destination device.
    pub destination_address: u16,

    /// The mask applied to the VBus address of the destination device.
    pub destination_mask: u16,

    /// The VBus address of the source device.
    pub source_address: u16,

    /// The mask applied to the VBus address of the source device.
    pub source_mask: u16,

    /// The VBus command of the packet.
    pub command: u16,

    /// The list of fields contained in the frame data payload.
    pub fields: Vec<PacketTemplateField>,
}

/// A type to describe different data types within the packet fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    /// Floating-point number, supporting precision and an optional unit.
    Number,

    /// Time as hours and minutes: "HH:MM".
    Time,

    /// Date and time as weekday, hours and minutes: "DDD,HH:MM".
    WeekTime,

    /// Date and time: "YYYY-MM-DD HH:MM:SS"
    DateTime,
}

/// A numeric reference to a `Type` instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeId(pub i32);

/// Contains information about a field with the frame data payload of a VBus packet.
#[derive(Clone, Debug)]
pub struct PacketTemplateField {
    /// The `TextIndex` of the field's ID.
    pub id_text_index: TextIndex,

    /// The `LocalizedTextIndex` of the field's name.
    pub name_localized_text_index: LocalizedTextIndex,

    /// The `UnitId` of the field.
    pub unit_id: UnitId,

    /// The number of fractional digits.
    pub precision: i32,

    /// The `TypeId` of the field.
    pub type_id: TypeId,

    /// The list of parts that make up the field's value.
    pub parts: Vec<PacketTemplateFieldPart>,
}

/// Contains information about one part of a packet field's raw value.
#[derive(Clone, Debug, PartialEq)]
pub struct PacketTemplateFieldPart {
    /// The offset into the frame data payload.
    pub offset: i32,

    /// The bit position from which the part starts.
    pub bit_pos: u8,

    /// The bit mask that is applied to this part's value.
    pub mask: u8,

    /// Whether this part is signed (= sign-extended) or not (= zero-extended).
    pub is_signed: bool,

    /// The factor this part is multiplied with.
    pub factor: i64,
}

/// Contains the information from a VSF1 file.
#[derive(Debug)]
pub struct SpecificationFile {
    /// Date of VSF creation in format 'YYYMMDD'
    pub datecode: i32,

    /// List of texts.
    pub texts: Vec<String>,

    /// List of localized texts.
    pub localized_texts: Vec<LocalizedText>,

    /// List of units.
    pub units: Vec<Unit>,

    /// List of device templates.
    pub device_templates: Vec<DeviceTemplate>,

    /// List of packet templates.
    pub packet_templates: Vec<PacketTemplate>,
}

impl SpecificationFile {
    /// Construct a new `SpecificationFile` from a byte slice of VSF1 data.
    pub fn from_bytes(bytes: &[u8]) -> Result<SpecificationFile> {
        let texts = Vec::<String>::new();
        let localized_texts = Vec::<LocalizedText>::new();
        let units = Vec::<Unit>::new();
        let device_templates = Vec::<DeviceTemplate>::new();
        let packet_templates = Vec::<PacketTemplate>::new();

        let mut spec_file = SpecificationFile {
            datecode: 0,
            texts,
            localized_texts,
            units,
            device_templates,
            packet_templates,
        };

        if !check_offset(bytes, 0, 0x10, 1) {
            err(ErrorKind::InvalidFileHeader)
        } else {
            let fileheader = slice_entry(bytes, 0, 0x10);
            let checksum_a = LittleEndian::read_u16(&fileheader[0x00..0x02]);
            let checksum_b = LittleEndian::read_u16(&fileheader[0x02..0x04]);
            let total_length = LittleEndian::read_i32(&fileheader[0x04..0x08]) as usize;
            let data_version = LittleEndian::read_i32(&fileheader[0x08..0x0C]);
            let specification_offset = LittleEndian::read_i32(&fileheader[0x0C..0x10]) as usize;

            if total_length != bytes.len() {
                err(ErrorKind::InvalidFileHeaderTotalLength)
            } else if calc_crc16(&bytes[0x04..total_length]) != checksum_a {
                err(ErrorKind::InvalidFileHeaderChecksumA)
            } else if checksum_a != checksum_b {
                err(ErrorKind::InvalidFileHeaderChecksumB)
            } else if data_version != 1 {
                err(ErrorKind::InvalidFileHeaderDataVersion)
            } else if !check_offset(bytes, specification_offset, 0x2C, 1) {
                err(ErrorKind::InvalidFileHeaderSpecificationOffset)
            } else {
                spec_file.parse_specification_block(bytes, specification_offset)?;
                Ok(spec_file)
            }
        }
    }

    /// Construct a new `SpecificationFile` from the embedded default VSF data.
    pub fn new_default() -> SpecificationFile {
        Self::from_bytes(include_bytes!("../res/vbus_specification.vsf")).unwrap()
    }

    /// Get text by its index.
    pub fn text_by_index(&self, idx: &TextIndex) -> &str {
        let text = &self.texts[idx.0 as usize];
        text.as_str()
    }

    /// Get localized text by its index and language.
    pub fn localized_text_by_index(&self, idx: &LocalizedTextIndex, language: Language) -> &str {
        let localized_text = &self.localized_texts[idx.0 as usize];
        let text_index = match language {
            Language::En => &localized_text.text_index_en,
            Language::De => &localized_text.text_index_de,
            Language::Fr => &localized_text.text_index_fr,
        };
        self.text_by_index(text_index)
    }

    /// Get `UnitFamily` by its ID.
    pub fn unit_family_by_id(&self, id: &UnitFamilyId) -> UnitFamily {
        match id.0 {
            -1 => UnitFamily::None,
            0 => UnitFamily::Temperature,
            1 => UnitFamily::Energy,
            2 => UnitFamily::VolumeFlow,
            3 => UnitFamily::Pressure,
            4 => UnitFamily::Volume,
            5 => UnitFamily::Time,
            6 => UnitFamily::Power,
            _ => panic!("Unsupported unit family ID {:?}", id),
        }
    }

    /// Get `Unit` by its index.
    pub fn unit_by_id(&self, id: &UnitId) -> &Unit {
        self.units.iter().find(|&unit| &unit.unit_id == id).unwrap()
    }

    /// Get `Type` by its ID.
    pub fn type_by_id(&self, id: &TypeId) -> Type {
        match id.0 {
            1 => Type::Number,
            3 => Type::Time,
            4 => Type::WeekTime,
            5 => Type::DateTime,
            _ => panic!("Unsupported type ID {:?}", id),
        }
    }

    /// Find a `DeviceTemplate` matching the self and peer addresses.
    pub fn find_device_template(
        &self,
        self_address: u16,
        peer_address: u16,
    ) -> Option<&DeviceTemplate> {
        self.device_templates.iter().find(|&device_template| {
            let self_valid =
                ((device_template.self_address ^ self_address) & device_template.self_mask) == 0;
            let peer_valid =
                ((device_template.peer_address ^ peer_address) & device_template.peer_mask) == 0;
            self_valid && peer_valid
        })
    }

    /// Find a `PacketTemplate` matching the destination and source addresses as well as the command.
    pub fn find_packet_template(
        &self,
        destination_address: u16,
        source_address: u16,
        command: u16,
    ) -> Option<&PacketTemplate> {
        self.packet_templates.iter().find(|&packet_template| {
            let dst_valid = ((packet_template.destination_address ^ destination_address)
                & packet_template.destination_mask)
                == 0;
            let src_valid = ((packet_template.source_address ^ source_address)
                & packet_template.source_mask)
                == 0;
            dst_valid && src_valid && (packet_template.command == command)
        })
    }

    fn check_text_index(&self, idx: i32) -> bool {
        (idx as usize) < self.texts.len()
    }

    fn check_localized_text_index(&self, idx: i32) -> bool {
        (idx as usize) < self.localized_texts.len()
    }

    fn check_unit_family_id(&self, id: i32) -> bool {
        (id >= -1) && (id <= 6)
    }

    fn check_unit_id(&self, id: i32) -> bool {
        self.units.iter().any(|unit| unit.unit_id.0 == id)
    }

    fn check_type_id(&self, id: i32) -> bool {
        match id {
            1 | 3 | 4 | 5 => true,
            _ => false,
        }
    }

    fn parse_specification_block(&mut self, bytes: &[u8], offset: usize) -> Result<()> {
        let block = slice_entry(bytes, offset, 0x2C);
        let datecode = LittleEndian::read_i32(&block[0x00..0x04]);
        let text_count = LittleEndian::read_i32(&block[0x04..0x08]) as usize;
        let text_table_offset = LittleEndian::read_i32(&block[0x08..0x0C]) as usize;
        let localized_text_count = LittleEndian::read_i32(&block[0x0C..0x10]) as usize;
        let localized_text_table_offset = LittleEndian::read_i32(&block[0x10..0x14]) as usize;
        let unit_count = LittleEndian::read_i32(&block[0x14..0x18]) as usize;
        let unit_table_offset = LittleEndian::read_i32(&block[0x18..0x1C]) as usize;
        let device_template_count = LittleEndian::read_i32(&block[0x1C..0x20]) as usize;
        let device_template_table_offset = LittleEndian::read_i32(&block[0x20..0x24]) as usize;
        let packet_template_count = LittleEndian::read_i32(&block[0x24..0x28]) as usize;
        let packet_template_table_offset = LittleEndian::read_i32(&block[0x28..0x2C]) as usize;

        if !check_offset(bytes, text_table_offset, 0x04, text_count) {
            err(ErrorKind::InvalidSpecificationTextTable)
        } else if !check_offset(
            bytes,
            localized_text_table_offset,
            0x0C,
            localized_text_count,
        ) {
            err(ErrorKind::InvalidSpecificationLocalizedTextTable)
        } else if !check_offset(bytes, unit_table_offset, 0x10, unit_count) {
            err(ErrorKind::InvalidSpecificationUnitTable)
        } else if !check_offset(
            bytes,
            device_template_table_offset,
            0x0C,
            device_template_count,
        ) {
            err(ErrorKind::InvalidSpecificationDeviceTemplateTable)
        } else if !check_offset(
            bytes,
            packet_template_table_offset,
            0x14,
            packet_template_count,
        ) {
            err(ErrorKind::InvalidSpecificationPacketTemplateTable)
        } else {
            self.datecode = datecode;

            for index in 0..text_count {
                let text = self.parse_text_block(bytes, text_table_offset, index)?;
                self.texts.push(text);
            }

            for index in 0..localized_text_count {
                let localized_text =
                    self.parse_localized_text_block(bytes, localized_text_table_offset, index)?;
                self.localized_texts.push(localized_text);
            }

            for index in 0..unit_count {
                let unit = self.parse_unit_block(bytes, unit_table_offset, index)?;
                self.units.push(unit);
            }

            for index in 0..device_template_count {
                let device_template =
                    self.parse_device_template_block(bytes, device_template_table_offset, index)?;
                self.device_templates.push(device_template);
            }

            for index in 0..packet_template_count {
                let packet_template =
                    self.parse_packet_template_block(bytes, packet_template_table_offset, index)?;
                self.packet_templates.push(packet_template);
            }

            Ok(())
        }
    }

    fn parse_text_block(&mut self, bytes: &[u8], offset: usize, index: usize) -> Result<String> {
        use std::str;

        let block = slice_table_entry(bytes, offset, 0x04, index);
        let string_offset = LittleEndian::read_i32(&block[0x00..0x04]) as usize;

        if !check_offset(bytes, string_offset, 0x01, 1) {
            err(ErrorKind::InvalidTextStringOffset)
        } else {
            let mut string_end = string_offset;
            while string_end < bytes.len() && bytes[string_end] != 0 {
                string_end += 1;
            }
            match str::from_utf8(&bytes[string_offset..string_end]) {
                Ok(string) => Ok(string.to_string()),
                Err(_) => err(ErrorKind::InvalidTextContent),
            }
        }
    }

    fn parse_localized_text_block(
        &mut self,
        bytes: &[u8],
        offset: usize,
        index: usize,
    ) -> Result<LocalizedText> {
        let block = slice_table_entry(bytes, offset, 0x0C, index);
        let text_index_en = LittleEndian::read_i32(&block[0x00..0x04]);
        let text_index_de = LittleEndian::read_i32(&block[0x04..0x08]);
        let text_index_fr = LittleEndian::read_i32(&block[0x08..0x0C]);

        if !self.check_text_index(text_index_en) {
            err(ErrorKind::InvalidLocalizedTextTextIndexEn)
        } else if !self.check_text_index(text_index_de) {
            err(ErrorKind::InvalidLocalizedTextTextIndexDe)
        } else if !self.check_text_index(text_index_fr) {
            err(ErrorKind::InvalidLocalizedTextTextIndexFr)
        } else {
            Ok(LocalizedText {
                text_index_en: TextIndex(text_index_en),
                text_index_de: TextIndex(text_index_de),
                text_index_fr: TextIndex(text_index_fr),
            })
        }
    }

    fn parse_unit_block(&mut self, bytes: &[u8], offset: usize, index: usize) -> Result<Unit> {
        let block = slice_table_entry(bytes, offset, 0x10, index);
        let unit_id = LittleEndian::read_i32(&block[0x00..0x04]);
        let unit_family_id = LittleEndian::read_i32(&block[0x04..0x08]);
        let unit_code_text_index = LittleEndian::read_i32(&block[0x08..0x0C]);
        let unit_text_text_index = LittleEndian::read_i32(&block[0x0C..0x10]);

        if !self.check_unit_family_id(unit_family_id) {
            err(ErrorKind::InvalidUnitUnitFamilyId)
        } else if !self.check_text_index(unit_code_text_index) {
            err(ErrorKind::InvalidUnitUnitCodeTextIndex)
        } else if !self.check_text_index(unit_text_text_index) {
            err(ErrorKind::InvalidUnitUnitTextTextIndex)
        } else {
            Ok(Unit {
                unit_id: UnitId(unit_id),
                unit_family_id: UnitFamilyId(unit_family_id),
                unit_code_text_index: TextIndex(unit_code_text_index),
                unit_text_text_index: TextIndex(unit_text_text_index),
            })
        }
    }

    fn parse_device_template_block(
        &mut self,
        bytes: &[u8],
        offset: usize,
        index: usize,
    ) -> Result<DeviceTemplate> {
        let block = slice_table_entry(bytes, offset, 0x0C, index);
        let self_address = LittleEndian::read_u16(&block[0x00..0x02]);
        let self_mask = LittleEndian::read_u16(&block[0x02..0x04]);
        let peer_address = LittleEndian::read_u16(&block[0x04..0x06]);
        let peer_mask = LittleEndian::read_u16(&block[0x06..0x08]);
        let name_localized_text_index = LittleEndian::read_i32(&block[0x08..0x0C]);

        if !self.check_localized_text_index(name_localized_text_index) {
            err(ErrorKind::InvalidDeviceTemplateNameLocalizedTextIndex)
        } else {
            Ok(DeviceTemplate {
                self_address,
                self_mask,
                peer_address,
                peer_mask,
                name_localized_text_index: LocalizedTextIndex(name_localized_text_index),
            })
        }
    }

    fn parse_packet_template_block(
        &mut self,
        bytes: &[u8],
        offset: usize,
        index: usize,
    ) -> Result<PacketTemplate> {
        let block = slice_table_entry(bytes, offset, 0x14, index);
        let destination_address = LittleEndian::read_u16(&block[0x00..0x02]);
        let destination_mask = LittleEndian::read_u16(&block[0x02..0x04]);
        let source_address = LittleEndian::read_u16(&block[0x04..0x06]);
        let source_mask = LittleEndian::read_u16(&block[0x06..0x08]);
        let command = LittleEndian::read_u16(&block[0x08..0x0A]);
        let field_count = LittleEndian::read_i32(&block[0x0C..0x10]) as usize;
        let field_table_offset = LittleEndian::read_i32(&block[0x10..0x14]) as usize;

        if !check_offset(bytes, field_table_offset, 0x1C, field_count) {
            err(ErrorKind::InvalidPacketTemplateFieldTable)
        } else {
            let mut fields = Vec::<PacketTemplateField>::with_capacity(field_count);
            for index in 0..field_count {
                let field =
                    self.parse_packet_template_field_block(bytes, field_table_offset, index)?;
                fields.push(field);
            }

            Ok(PacketTemplate {
                destination_address,
                destination_mask,
                source_address,
                source_mask,
                command,
                fields,
            })
        }
    }

    fn parse_packet_template_field_block(
        &mut self,
        bytes: &[u8],
        offset: usize,
        index: usize,
    ) -> Result<PacketTemplateField> {
        let block = slice_table_entry(bytes, offset, 0x1C, index);
        let id_text_index = LittleEndian::read_i32(&block[0x00..0x04]);
        let name_localized_text_index = LittleEndian::read_i32(&block[0x04..0x08]);
        let unit_id = LittleEndian::read_i32(&block[0x08..0x0C]);
        let precision = LittleEndian::read_i32(&block[0x0C..0x10]);
        let type_id = LittleEndian::read_i32(&block[0x10..0x14]);
        let part_count = LittleEndian::read_i32(&block[0x14..0x18]) as usize;
        let part_table_offset = LittleEndian::read_i32(&block[0x18..0x1C]) as usize;

        if !self.check_text_index(id_text_index) {
            err(ErrorKind::InvalidPacketTemplateFieldIdTextIndex)
        } else if !self.check_localized_text_index(name_localized_text_index) {
            err(ErrorKind::InvalidPacketTemplateFieldNameLocalizedTextIndex)
        } else if !self.check_unit_id(unit_id) {
            err(ErrorKind::InvalidPacketTemplateFieldUnitId)
        } else if !self.check_type_id(type_id) {
            err(ErrorKind::InvalidPacketTemplateFieldTypeId)
        } else if !check_offset(bytes, part_table_offset, 0x10, part_count) {
            err(ErrorKind::InvalidPacketTemplateFieldPartTable)
        } else {
            let mut parts = Vec::<PacketTemplateFieldPart>::with_capacity(part_count);
            for index in 0..part_count {
                let part =
                    self.parse_packet_template_field_part_block(bytes, part_table_offset, index)?;
                parts.push(part);
            }

            Ok(PacketTemplateField {
                id_text_index: TextIndex(id_text_index),
                name_localized_text_index: LocalizedTextIndex(name_localized_text_index),
                unit_id: UnitId(unit_id),
                precision,
                type_id: TypeId(type_id),
                parts,
            })
        }
    }

    fn parse_packet_template_field_part_block(
        &mut self,
        bytes: &[u8],
        offset: usize,
        index: usize,
    ) -> Result<PacketTemplateFieldPart> {
        let block = slice_table_entry(bytes, offset, 0x10, index);
        let data_offset = LittleEndian::read_i32(&block[0x00..0x04]);
        let bit_pos = block[0x04];
        let mask = block[0x05];
        let is_signed = block[0x06];
        let factor = LittleEndian::read_i64(&block[0x08..0x10]);

        Ok(PacketTemplateFieldPart {
            offset: data_offset,
            bit_pos,
            mask,
            is_signed: is_signed != 0,
            factor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_data::SPEC_FILE_1;

    fn check_spec_file_fixture(spec_file: &SpecificationFile) {
        let mut text_index = 0;

        let mut check_next_text = |ref_text| {
            let text = spec_file.text_by_index(&TextIndex(text_index));
            assert_eq!(ref_text, text);
            text_index += 1;
        };

        assert_eq!(188, spec_file.texts.len());
        check_next_text("");
        check_next_text(" BTU");
        check_next_text(" Hz");
        check_next_text(" K");
        check_next_text(" MBTU");
        check_next_text(" MMBTU");
        check_next_text(" MWh");
        check_next_text(" V");
        check_next_text(" W");
        check_next_text(" W/m²");
        check_next_text(" Wh");
        check_next_text(" bar");
        check_next_text(" d");
        check_next_text(" g CO₂ (Gas)");
        check_next_text(" g CO₂ (Oil)");
        check_next_text(" gal");
        check_next_text(" gal/h");
        check_next_text(" gal/min");
        check_next_text(" h");
        check_next_text(" hPa");
        check_next_text(" kW");
        check_next_text(" kWh");
        check_next_text(" kWh/(m²*d)");
        check_next_text(" kg CO₂ (Gas)");
        check_next_text(" kg CO₂ (Oil)");
        check_next_text(" kg/h");
        check_next_text(" kg/m³");
        check_next_text(" l");
        check_next_text(" l/(m²*d)");
        check_next_text(" l/h");
        check_next_text(" l/min");
        check_next_text(" m/s");
        check_next_text(" mA");
        check_next_text(" min");
        check_next_text(" ms");
        check_next_text(" m²");
        check_next_text(" m³");
        check_next_text(" m³/h");
        check_next_text(" psi");
        check_next_text(" s");
        check_next_text(" t CO₂ (Gas)");
        check_next_text(" t CO₂ (Oil)");
        check_next_text(" °");
        check_next_text(" °C");
        check_next_text(" °F");
        check_next_text(" µV");
        check_next_text(" \u{2126}"); // OHM SIGN
        check_next_text("%");
        check_next_text("000_4_0");
        check_next_text("004_4_0");
        check_next_text("008_4_0");
        check_next_text("012_4_0");
        check_next_text("016_4_0");
        check_next_text("020_4_0");
        check_next_text("024_4_0");
        check_next_text("028_4_0");
        check_next_text("032_4_0");
        check_next_text("036_4_0");
        check_next_text("040_4_0");
        check_next_text("044_4_0");
        check_next_text("048_4_0");
        check_next_text("052_4_0");
        check_next_text("056_4_0");
        check_next_text("060_4_0");
        check_next_text("064_4_0");
        check_next_text("068_2_0");
        check_next_text("5 min error code");
        check_next_text("5-Min-Fehlercode");
        check_next_text("Bars");
        check_next_text("Btus");
        check_next_text("Chaleur solaire");
        check_next_text("Code erreur 5 min");
        check_next_text("CubicMeters");
        check_next_text("CubicMetersPerHour");
        check_next_text("DFA");
        check_next_text("Date measured values");
        check_next_text("Date valeurs de mesure");
        check_next_text("Datum_Messdaten");
        check_next_text("Days");
        check_next_text("DegreesAngular");
        check_next_text("DegreesCelsius");
        check_next_text("DegreesFahrenheit");
        check_next_text("DegreesKelvin");
        check_next_text("DeltaSol MX [WMZ #0]");
        check_next_text("DeltaSol MX [WMZ #10]");
        check_next_text("DeltaSol MX [WMZ #11]");
        check_next_text("DeltaSol MX [WMZ #12]");
        check_next_text("DeltaSol MX [WMZ #13]");
        check_next_text("DeltaSol MX [WMZ #14]");
        check_next_text("DeltaSol MX [WMZ #15]");
        check_next_text("DeltaSol MX [WMZ #1]");
        check_next_text("DeltaSol MX [WMZ #2]");
        check_next_text("DeltaSol MX [WMZ #3]");
        check_next_text("DeltaSol MX [WMZ #4]");
        check_next_text("DeltaSol MX [WMZ #5]");
        check_next_text("DeltaSol MX [WMZ #6]");
        check_next_text("DeltaSol MX [WMZ #7]");
        check_next_text("DeltaSol MX [WMZ #8]");
        check_next_text("DeltaSol MX [WMZ #9]");
        check_next_text("DeltaSol MX [WMZ #]");
        check_next_text("Einstrahlung");
        check_next_text("Gallons");
        check_next_text("GallonsPerHour");
        check_next_text("GallonsPerMinute");
        check_next_text("Gesamtvolumen");
        check_next_text("GramsCO2Gas");
        check_next_text("GramsCO2Oil");
        check_next_text("Heat quantity");
        check_next_text("Heat quantity 1");
        check_next_text("Heat quantity 2");
        check_next_text("Heat quantity today");
        check_next_text("Heat quantity week");
        check_next_text("Hectopascals");
        check_next_text("Hertz");
        check_next_text("Hours");
        check_next_text("IOC-Modul [Messwerte]");
        check_next_text("Intensité courant 1");
        check_next_text("Intensité courant 2");
        check_next_text("Irradiation");
        check_next_text("KiloBtus");
        check_next_text("KiloWattHoursPerSquareMeterPerDay");
        check_next_text("KilogramsCO2Gas");
        check_next_text("KilogramsCO2Oil");
        check_next_text("KilogramsPerCubicMeter");
        check_next_text("KilogramsPerHour");
        check_next_text("KilowattHours");
        check_next_text("Kilowatts");
        check_next_text("Liters");
        check_next_text("LitersPerHour");
        check_next_text("LitersPerMinute");
        check_next_text("LitersPerSquareMeterPerDay");
        check_next_text("MegaBtus");
        check_next_text("MegawattHours");
        check_next_text("MetersPerSecond");
        check_next_text("Microvolts");
        check_next_text("Milliamperes");
        check_next_text("Milliseconds");
        check_next_text("Minutes");
        check_next_text("None");
        check_next_text("N° secondes");
        check_next_text("Ohms");
        check_next_text("Percent");
        check_next_text("PoundsForcePerSquareInch");
        check_next_text("Quantité de chaleur");
        check_next_text("Quantité de chaleur 1");
        check_next_text("Quantité de chaleur 2");
        check_next_text("Quantité de chaleur aujourd\'hui");
        check_next_text("Quantité de chaleur semaine");
        check_next_text("Rated current 1");
        check_next_text("Rated current 2");
        check_next_text("S6");
        check_next_text("S7");
        check_next_text("Seconds");
        check_next_text("Seconds no.");
        check_next_text("SekNr");
        check_next_text("Solar heat");
        check_next_text("Solarwärme");
        check_next_text("SquareMeters");
        check_next_text("Stromstärke 1");
        check_next_text("Stromstärke 2");
        check_next_text("T- Départ / S1");
        check_next_text("T-Ambiance");
        check_next_text("T-Retour /S2");
        check_next_text("T-Rücklauf/S2");
        check_next_text("T-Umgebung");
        check_next_text("T-Vorlauf/S1");
        check_next_text("T-ambient");
        check_next_text("T-flow / S1");
        check_next_text("T-return / S2");
        check_next_text("TSL");
        check_next_text("Tmax-Temp_/S5");
        check_next_text("TonsCO2Gas");
        check_next_text("TonsCO2Oil");
        check_next_text("Volts");
        check_next_text("Volumen Monat");
        check_next_text("Volumen Woche");
        check_next_text("Volumen heute");
        check_next_text("Volumenstr_1");
        check_next_text("Volumenstr_2");
        check_next_text("WattHours");
        check_next_text("Watts");
        check_next_text("WattsPerSquareMeter");
        check_next_text("Wärmemenge");
        check_next_text("Wärmemenge 1");
        check_next_text("Wärmemenge 2");
        check_next_text("Wärmemenge Monat");
        check_next_text("Wärmemenge Woche");
        check_next_text("Wärmemenge heute");

        let mut localized_text_index = 0;

        let mut check_next_localized_text = |ref_text_en, ref_text_de, ref_text_fr| {
            let text = spec_file
                .localized_text_by_index(&LocalizedTextIndex(localized_text_index), Language::En);
            assert_eq!(ref_text_en, text);
            let text = spec_file
                .localized_text_by_index(&LocalizedTextIndex(localized_text_index), Language::De);
            assert_eq!(ref_text_de, text);
            let text = spec_file
                .localized_text_by_index(&LocalizedTextIndex(localized_text_index), Language::Fr);
            assert_eq!(ref_text_fr, text);
            localized_text_index += 1;
        };

        assert_eq!(45, spec_file.localized_texts.len());
        check_next_localized_text("5 min error code", "5-Min-Fehlercode", "Code erreur 5 min");
        check_next_localized_text("DFA", "DFA", "DFA");
        check_next_localized_text(
            "Date measured values",
            "Datum_Messdaten",
            "Date valeurs de mesure",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #0]",
            "DeltaSol MX [WMZ #0]",
            "DeltaSol MX [WMZ #0]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #10]",
            "DeltaSol MX [WMZ #10]",
            "DeltaSol MX [WMZ #10]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #11]",
            "DeltaSol MX [WMZ #11]",
            "DeltaSol MX [WMZ #11]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #12]",
            "DeltaSol MX [WMZ #12]",
            "DeltaSol MX [WMZ #12]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #13]",
            "DeltaSol MX [WMZ #13]",
            "DeltaSol MX [WMZ #13]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #14]",
            "DeltaSol MX [WMZ #14]",
            "DeltaSol MX [WMZ #14]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #15]",
            "DeltaSol MX [WMZ #15]",
            "DeltaSol MX [WMZ #15]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #1]",
            "DeltaSol MX [WMZ #1]",
            "DeltaSol MX [WMZ #1]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #2]",
            "DeltaSol MX [WMZ #2]",
            "DeltaSol MX [WMZ #2]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #3]",
            "DeltaSol MX [WMZ #3]",
            "DeltaSol MX [WMZ #3]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #4]",
            "DeltaSol MX [WMZ #4]",
            "DeltaSol MX [WMZ #4]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #5]",
            "DeltaSol MX [WMZ #5]",
            "DeltaSol MX [WMZ #5]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #6]",
            "DeltaSol MX [WMZ #6]",
            "DeltaSol MX [WMZ #6]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #7]",
            "DeltaSol MX [WMZ #7]",
            "DeltaSol MX [WMZ #7]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #8]",
            "DeltaSol MX [WMZ #8]",
            "DeltaSol MX [WMZ #8]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #9]",
            "DeltaSol MX [WMZ #9]",
            "DeltaSol MX [WMZ #9]",
        );
        check_next_localized_text(
            "DeltaSol MX [WMZ #]",
            "DeltaSol MX [WMZ #]",
            "DeltaSol MX [WMZ #]",
        );
        check_next_localized_text("Irradiation", "Einstrahlung", "Irradiation");
        check_next_localized_text("Gesamtvolumen", "Gesamtvolumen", "Gesamtvolumen");
        check_next_localized_text(
            "IOC-Modul [Messwerte]",
            "IOC-Modul [Messwerte]",
            "IOC-Modul [Messwerte]",
        );
        check_next_localized_text("S6", "S6", "S6");
        check_next_localized_text("S7", "S7", "S7");
        check_next_localized_text("Seconds no.", "SekNr", "N° secondes");
        check_next_localized_text("Solar heat", "Solarwärme", "Chaleur solaire");
        check_next_localized_text("Rated current 1", "Stromstärke 1", "Intensité courant 1");
        check_next_localized_text("Rated current 2", "Stromstärke 2", "Intensité courant 2");
        check_next_localized_text("T-return / S2", "T-Rücklauf/S2", "T-Retour /S2");
        check_next_localized_text("T-ambient", "T-Umgebung", "T-Ambiance");
        check_next_localized_text("T-flow / S1", "T-Vorlauf/S1", "T- Départ / S1");
        check_next_localized_text("TSL", "TSL", "TSL");
        check_next_localized_text("Tmax-Temp_/S5", "Tmax-Temp_/S5", "Tmax-Temp_/S5");
        check_next_localized_text("Volumen Monat", "Volumen Monat", "Volumen Monat");
        check_next_localized_text("Volumen Woche", "Volumen Woche", "Volumen Woche");
        check_next_localized_text("Volumen heute", "Volumen heute", "Volumen heute");
        check_next_localized_text("Volumenstr_1", "Volumenstr_1", "Volumenstr_1");
        check_next_localized_text("Volumenstr_2", "Volumenstr_2", "Volumenstr_2");
        check_next_localized_text("Heat quantity", "Wärmemenge", "Quantité de chaleur");
        check_next_localized_text("Heat quantity 1", "Wärmemenge 1", "Quantité de chaleur 1");
        check_next_localized_text("Heat quantity 2", "Wärmemenge 2", "Quantité de chaleur 2");
        check_next_localized_text(
            "Wärmemenge Monat",
            "Wärmemenge Monat",
            "Wärmemenge Monat",
        );
        check_next_localized_text(
            "Heat quantity week",
            "Wärmemenge Woche",
            "Quantité de chaleur semaine",
        );
        check_next_localized_text(
            "Heat quantity today",
            "Wärmemenge heute",
            "Quantité de chaleur aujourd'hui",
        );

        let mut unit_index = 0;

        let mut check_next_unit = |unit_id, unit_family, unit_code, unit_text| {
            let unit = &spec_file.units[unit_index];

            assert_eq!(UnitId(unit_id), unit.unit_id);
            assert_eq!(
                unit_family,
                spec_file.unit_family_by_id(&unit.unit_family_id)
            );
            assert_eq!(
                unit_code,
                spec_file.text_by_index(&unit.unit_code_text_index)
            );
            assert_eq!(
                unit_text,
                spec_file.text_by_index(&unit.unit_text_text_index)
            );

            unit_index += 1;
        };

        assert_eq!(48, spec_file.units.len());
        check_next_unit(55, UnitFamily::Pressure, "Bars", " bar");
        check_next_unit(20, UnitFamily::Energy, "Btus", " BTU");
        check_next_unit(80, UnitFamily::Volume, "CubicMeters", " m³");
        check_next_unit(135, UnitFamily::VolumeFlow, "CubicMetersPerHour", " m³/h");
        check_next_unit(70, UnitFamily::None, "Days", " d");
        check_next_unit(90, UnitFamily::None, "DegreesAngular", " °");
        check_next_unit(62, UnitFamily::Temperature, "DegreesCelsius", " °C");
        check_next_unit(64, UnitFamily::Temperature, "DegreesFahrenheit", " °F");
        check_next_unit(63, UnitFamily::None, "DegreesKelvin", " K");
        check_next_unit(1042, UnitFamily::Volume, "Gallons", " gal");
        check_next_unit(1041, UnitFamily::VolumeFlow, "GallonsPerHour", " gal/h");
        check_next_unit(1040, UnitFamily::VolumeFlow, "GallonsPerMinute", " gal/min");
        check_next_unit(1035, UnitFamily::Energy, "GramsCO2Gas", " g CO₂ (Gas)");
        check_next_unit(1032, UnitFamily::Energy, "GramsCO2Oil", " g CO₂ (Oil)");
        check_next_unit(133, UnitFamily::None, "Hectopascals", " hPa");
        check_next_unit(27, UnitFamily::None, "Hertz", " Hz");
        check_next_unit(71, UnitFamily::None, "Hours", " h");
        check_next_unit(1030, UnitFamily::Energy, "KiloBtus", " MBTU");
        check_next_unit(
            1024,
            UnitFamily::None,
            "KiloWattHoursPerSquareMeterPerDay",
            " kWh/(m²*d)",
        );
        check_next_unit(
            1036,
            UnitFamily::Energy,
            "KilogramsCO2Gas",
            " kg CO₂ (Gas)",
        );
        check_next_unit(
            1033,
            UnitFamily::Energy,
            "KilogramsCO2Oil",
            " kg CO₂ (Oil)",
        );
        check_next_unit(186, UnitFamily::None, "KilogramsPerCubicMeter", " kg/m³");
        check_next_unit(44, UnitFamily::None, "KilogramsPerHour", " kg/h");
        check_next_unit(19, UnitFamily::Energy, "KilowattHours", " kWh");
        check_next_unit(48, UnitFamily::None, "Kilowatts", " kW");
        check_next_unit(82, UnitFamily::Volume, "Liters", " l");
        check_next_unit(136, UnitFamily::VolumeFlow, "LitersPerHour", " l/h");
        check_next_unit(88, UnitFamily::VolumeFlow, "LitersPerMinute", " l/min");
        check_next_unit(
            1025,
            UnitFamily::None,
            "LitersPerSquareMeterPerDay",
            " l/(m²*d)",
        );
        check_next_unit(1031, UnitFamily::Energy, "MegaBtus", " MMBTU");
        check_next_unit(146, UnitFamily::Energy, "MegawattHours", " MWh");
        check_next_unit(74, UnitFamily::None, "MetersPerSecond", " m/s");
        check_next_unit(1100, UnitFamily::None, "Microvolts", " µV");
        check_next_unit(2, UnitFamily::None, "Milliamperes", " mA");
        check_next_unit(159, UnitFamily::None, "Milliseconds", " ms");
        check_next_unit(72, UnitFamily::None, "Minutes", " min");
        check_next_unit(-1, UnitFamily::None, "None", "");
        check_next_unit(4, UnitFamily::None, "Ohms", " \u{2126}");
        check_next_unit(98, UnitFamily::None, "Percent", "%");
        check_next_unit(56, UnitFamily::Pressure, "PoundsForcePerSquareInch", " psi");
        check_next_unit(73, UnitFamily::None, "Seconds", " s");
        check_next_unit(0, UnitFamily::None, "SquareMeters", " m²");
        check_next_unit(1037, UnitFamily::Energy, "TonsCO2Gas", " t CO₂ (Gas)");
        check_next_unit(1034, UnitFamily::Energy, "TonsCO2Oil", " t CO₂ (Oil)");
        check_next_unit(5, UnitFamily::None, "Volts", " V");
        check_next_unit(18, UnitFamily::Energy, "WattHours", " Wh");
        check_next_unit(47, UnitFamily::None, "Watts", " W");
        check_next_unit(35, UnitFamily::None, "WattsPerSquareMeter", " W/m²");

        assert_eq!(18, spec_file.device_templates.len());

        let dt = &spec_file.device_templates[0];
        assert_eq!(0x0010, dt.self_address);
        assert_eq!(0xFFFF, dt.self_mask);
        assert_eq!(0x0000, dt.peer_address);
        assert_eq!(0x0000, dt.peer_mask);
        assert_eq!(
            "DFA",
            spec_file.localized_text_by_index(&dt.name_localized_text_index, Language::En)
        );

        assert_eq!(2, spec_file.packet_templates.len());

        let pt = &spec_file.packet_templates[0];
        assert_eq!(0x0010, pt.destination_address);
        assert_eq!(0xFFFF, pt.destination_mask);
        assert_eq!(0x7E30, pt.source_address);
        assert_eq!(0xFFF0, pt.source_mask);
        assert_eq!(0x0100, pt.command);
        assert_eq!(8, pt.fields.len());

        let pt = &spec_file.packet_templates[1];
        assert_eq!(0x0010, pt.destination_address);
        assert_eq!(0xFFFF, pt.destination_mask);
        assert_eq!(0x7F61, pt.source_address);
        assert_eq!(0xFFFF, pt.source_mask);
        assert_eq!(0x0100, pt.command);
        assert_eq!(18, pt.fields.len());

        let ptf = &spec_file.packet_templates[0].fields[0];
        assert_eq!("000_4_0", spec_file.text_by_index(&ptf.id_text_index));
        assert_eq!(
            "Heat quantity",
            spec_file.localized_text_by_index(&ptf.name_localized_text_index, Language::En)
        );
        assert_eq!(
            "Wärmemenge",
            spec_file.localized_text_by_index(&ptf.name_localized_text_index, Language::De)
        );
        assert_eq!(
            "Quantité de chaleur",
            spec_file.localized_text_by_index(&ptf.name_localized_text_index, Language::Fr)
        );
        assert_eq!(18, ptf.unit_id.0);
        assert_eq!(0, ptf.precision);
        assert_eq!(1, ptf.type_id.0);

        assert_eq!(8, ptf.parts.len());

        let mut part_index = 0;

        let mut check_next_part = |offset, bit_pos, mask, is_signed, factor| {
            let part = &ptf.parts[part_index];

            assert_eq!(offset, part.offset);
            assert_eq!(bit_pos, part.bit_pos);
            assert_eq!(mask, part.mask);
            assert_eq!(is_signed, part.is_signed);
            assert_eq!(factor, part.factor);

            part_index += 1;
        };

        check_next_part(0, 0, 0xFF, false, 1);
        check_next_part(1, 0, 0xFF, false, 256);
        check_next_part(2, 0, 0xFF, false, 65536);
        check_next_part(3, 0, 0xFF, true, 16777216);
        check_next_part(36, 0, 0xFF, false, 1000000000);
        check_next_part(37, 0, 0xFF, false, 256000000000);
        check_next_part(38, 0, 0xFF, false, 65536000000000);
        check_next_part(39, 0, 0xFF, true, 16777216000000000);
    }

    #[test]
    fn test_from_bytes() {
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        check_spec_file_fixture(&spec_file);
    }

    #[test]
    fn test_new_default() {
        let _spec_file = SpecificationFile::new_default();
    }
}
