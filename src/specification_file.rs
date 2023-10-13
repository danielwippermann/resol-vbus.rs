//! A module that parses the contents of a VBus Specification File Version 1 (VSF1).
//!
//! The VBus Specification File format is used to provide information about VBus Protocol Version
//! 1.x `Packet`s and their frame data payload.
//!
//! See the [RESOL VBus Specification File Format v1](http://danielwippermann.github.io/resol-vbus/vbus-specification-file-format-v1.html)
//! for details.
use crate::{
    error::{Error, Result},
    utils::calc_crc16, little_endian::{u16_from_le_bytes, i32_from_le_bytes, i64_from_le_bytes},
};

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
    Err(Error::new(format!("Unable to parse VSF: {kind:?}")))
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
#[derive(Clone, Debug)]
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

const BTUS_PER_WATT_HOUR: f64 = 3.412_128;
const GALLONS_PER_LITER: f64 = 0.264_172;
const GRAMS_CO2_GAS_PER_WATT_HOUR: f64 = 0.2536;
const GRAMS_CO2_OIL_PER_WATT_HOUR: f64 = 0.568;
const POUNDS_FORCE_PER_SQUARE_INCH_PER_BAR: f64 = 14.503_773_8;

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
            let checksum_a = u16_from_le_bytes(&fileheader[0x00..0x02]);
            let checksum_b = u16_from_le_bytes(&fileheader[0x02..0x04]);
            let total_length = i32_from_le_bytes(&fileheader[0x04..0x08]) as usize;
            let data_version = i32_from_le_bytes(&fileheader[0x08..0x0C]);
            let specification_offset = i32_from_le_bytes(&fileheader[0x0C..0x10]) as usize;

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
    #[cfg(not(feature = "no-default-spec"))]
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
            _ => panic!("Unsupported unit family ID {id:?}"),
        }
    }

    /// Get `Unit` by its index.
    pub fn unit_by_id(&self, id: &UnitId) -> &Unit {
        self.units.iter().find(|&unit| &unit.unit_id == id).unwrap()
    }

    /// Get `Unit` by unit code.
    pub fn unit_by_unit_code(&self, unit_code: &str) -> Option<&Unit> {
        self.units.iter().find(|&unit| {
            let current_unit_code = self.text_by_index(&unit.unit_code_text_index);
            current_unit_code == unit_code
        })
    }

    /// Get `Type` by its ID.
    pub fn type_by_id(&self, id: &TypeId) -> Type {
        match id.0 {
            1 => Type::Number,
            3 => Type::Time,
            4 => Type::WeekTime,
            5 => Type::DateTime,
            _ => panic!("Unsupported type ID {id:?}"),
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
        (-1..=6).contains(&id)
    }

    fn check_unit_id(&self, id: i32) -> bool {
        self.units.iter().any(|unit| unit.unit_id.0 == id)
    }

    fn check_type_id(&self, id: i32) -> bool {
        matches!(id, 1 | 3 | 4 | 5)
    }

    fn parse_specification_block(&mut self, bytes: &[u8], offset: usize) -> Result<()> {
        let block = slice_entry(bytes, offset, 0x2C);
        let datecode = i32_from_le_bytes(&block[0x00..0x04]);
        let text_count = i32_from_le_bytes(&block[0x04..0x08]) as usize;
        let text_table_offset = i32_from_le_bytes(&block[0x08..0x0C]) as usize;
        let localized_text_count = i32_from_le_bytes(&block[0x0C..0x10]) as usize;
        let localized_text_table_offset = i32_from_le_bytes(&block[0x10..0x14]) as usize;
        let unit_count = i32_from_le_bytes(&block[0x14..0x18]) as usize;
        let unit_table_offset = i32_from_le_bytes(&block[0x18..0x1C]) as usize;
        let device_template_count = i32_from_le_bytes(&block[0x1C..0x20]) as usize;
        let device_template_table_offset = i32_from_le_bytes(&block[0x20..0x24]) as usize;
        let packet_template_count = i32_from_le_bytes(&block[0x24..0x28]) as usize;
        let packet_template_table_offset = i32_from_le_bytes(&block[0x28..0x2C]) as usize;

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
        let block = slice_table_entry(bytes, offset, 0x04, index);
        let string_offset = i32_from_le_bytes(&block[0x00..0x04]) as usize;

        if !check_offset(bytes, string_offset, 0x01, 1) {
            err(ErrorKind::InvalidTextStringOffset)
        } else {
            let mut string_end = string_offset;
            while string_end < bytes.len() && bytes[string_end] != 0 {
                string_end += 1;
            }
            match std::str::from_utf8(&bytes[string_offset..string_end]) {
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
        let text_index_en = i32_from_le_bytes(&block[0x00..0x04]);
        let text_index_de = i32_from_le_bytes(&block[0x04..0x08]);
        let text_index_fr = i32_from_le_bytes(&block[0x08..0x0C]);

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
        let unit_id = i32_from_le_bytes(&block[0x00..0x04]);
        let unit_family_id = i32_from_le_bytes(&block[0x04..0x08]);
        let unit_code_text_index = i32_from_le_bytes(&block[0x08..0x0C]);
        let unit_text_text_index = i32_from_le_bytes(&block[0x0C..0x10]);

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
        let self_address = u16_from_le_bytes(&block[0x00..0x02]);
        let self_mask = u16_from_le_bytes(&block[0x02..0x04]);
        let peer_address = u16_from_le_bytes(&block[0x04..0x06]);
        let peer_mask = u16_from_le_bytes(&block[0x06..0x08]);
        let name_localized_text_index = i32_from_le_bytes(&block[0x08..0x0C]);

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
        let destination_address = u16_from_le_bytes(&block[0x00..0x02]);
        let destination_mask = u16_from_le_bytes(&block[0x02..0x04]);
        let source_address = u16_from_le_bytes(&block[0x04..0x06]);
        let source_mask = u16_from_le_bytes(&block[0x06..0x08]);
        let command = u16_from_le_bytes(&block[0x08..0x0A]);
        let field_count = i32_from_le_bytes(&block[0x0C..0x10]) as usize;
        let field_table_offset = i32_from_le_bytes(&block[0x10..0x14]) as usize;

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
        let id_text_index = i32_from_le_bytes(&block[0x00..0x04]);
        let name_localized_text_index = i32_from_le_bytes(&block[0x04..0x08]);
        let unit_id = i32_from_le_bytes(&block[0x08..0x0C]);
        let precision = i32_from_le_bytes(&block[0x0C..0x10]);
        let type_id = i32_from_le_bytes(&block[0x10..0x14]);
        let part_count = i32_from_le_bytes(&block[0x14..0x18]) as usize;
        let part_table_offset = i32_from_le_bytes(&block[0x18..0x1C]) as usize;

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
                    self.parse_packet_template_field_part_block(bytes, part_table_offset, index);
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
    ) -> PacketTemplateFieldPart {
        let block = slice_table_entry(bytes, offset, 0x10, index);
        let data_offset = i32_from_le_bytes(&block[0x00..0x04]);
        let bit_pos = block[0x04];
        let mask = block[0x05];
        let is_signed = block[0x06];
        let factor = i64_from_le_bytes(&block[0x08..0x10]);

        PacketTemplateFieldPart {
            offset: data_offset,
            bit_pos,
            mask,
            is_signed: is_signed != 0,
            factor,
        }
    }

    /// Convert a value from one `Unit` to another.
    pub fn convert_value(&self, value: f64, src_unit: &Unit, dst_unit: &Unit) -> Result<f64> {
        if src_unit.unit_family_id != dst_unit.unit_family_id {
            return Err("Unit families differ".into());
        }

        let unit_family = self.unit_family_by_id(&src_unit.unit_family_id);
        let src_unit_code = self.text_by_index(&src_unit.unit_code_text_index);
        let dst_unit_code = self.text_by_index(&dst_unit.unit_code_text_index);

        let value = match unit_family {
            UnitFamily::None => return Err("Cannot convert values with UnitFamily::None".into()),
            UnitFamily::Temperature => {
                let value = match src_unit_code {
                    "DegreesCelsius" => value,
                    "DegreesFahrenheit" => (value - 32.0) / 1.8,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                };
                match dst_unit_code {
                    "DegreesCelsius" => value,
                    "DegreesFahrenheit" => value * 1.8 + 32.0,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                }
            }
            UnitFamily::Energy => {
                let value = match src_unit_code {
                    "Btus" => value / BTUS_PER_WATT_HOUR,
                    "GramsCO2Gas" => value / GRAMS_CO2_GAS_PER_WATT_HOUR,
                    "GramsCO2Oil" => value / GRAMS_CO2_OIL_PER_WATT_HOUR,
                    "KiloBtus" => value / BTUS_PER_WATT_HOUR * 1000.0,
                    "KilogramsCO2Gas" => value * 1000.0 / GRAMS_CO2_GAS_PER_WATT_HOUR,
                    "KilogramsCO2Oil" => value * 1000.0 / GRAMS_CO2_OIL_PER_WATT_HOUR,
                    "KilowattHours" => value * 1000.0,
                    "MegaBtus" => value / BTUS_PER_WATT_HOUR * 1_000_000.0,
                    "MegawattHours" => value * 1_000_000.0,
                    "TonsCO2Gas" => value * 1_000_000.0 / GRAMS_CO2_GAS_PER_WATT_HOUR,
                    "TonsCO2Oil" => value * 1_000_000.0 / GRAMS_CO2_OIL_PER_WATT_HOUR,
                    "WattHours" => value,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                };
                match dst_unit_code {
                    "Btus" => value * BTUS_PER_WATT_HOUR,
                    "GramsCO2Gas" => value * GRAMS_CO2_GAS_PER_WATT_HOUR,
                    "GramsCO2Oil" => value * GRAMS_CO2_OIL_PER_WATT_HOUR,
                    "KiloBtus" => value * BTUS_PER_WATT_HOUR / 1000.0,
                    "KilogramsCO2Gas" => value * GRAMS_CO2_GAS_PER_WATT_HOUR / 1000.0,
                    "KilogramsCO2Oil" => value * GRAMS_CO2_OIL_PER_WATT_HOUR / 1000.0,
                    "KilowattHours" => value / 1000.0,
                    "MegaBtus" => value * BTUS_PER_WATT_HOUR / 1_000_000.0,
                    "MegawattHours" => value / 1_000_000.0,
                    "TonsCO2Gas" => value * GRAMS_CO2_GAS_PER_WATT_HOUR / 1_000_000.0,
                    "TonsCO2Oil" => value * GRAMS_CO2_OIL_PER_WATT_HOUR / 1_000_000.0,
                    "WattHours" => value,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                }
            }
            UnitFamily::VolumeFlow => {
                let value = match src_unit_code {
                    "CubicMetersPerHour" => value * 1000.0,
                    "GallonsPerHour" => value / GALLONS_PER_LITER,
                    "GallonsPerMinute" => value / GALLONS_PER_LITER * 60.0,
                    "LitersPerHour" => value,
                    "LitersPerMinute" => value * 60.0,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                };
                match dst_unit_code {
                    "CubicMetersPerHour" => value / 1000.0,
                    "GallonsPerHour" => value * GALLONS_PER_LITER,
                    "GallonsPerMinute" => value * GALLONS_PER_LITER / 60.0,
                    "LitersPerHour" => value,
                    "LitersPerMinute" => value / 60.0,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                }
            }
            UnitFamily::Pressure => {
                let value = match src_unit_code {
                    "Bars" => value,
                    "PoundsForcePerSquareInch" => value / POUNDS_FORCE_PER_SQUARE_INCH_PER_BAR,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                };
                match dst_unit_code {
                    "Bars" => value,
                    "PoundsForcePerSquareInch" => value * POUNDS_FORCE_PER_SQUARE_INCH_PER_BAR,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                }
            }
            UnitFamily::Volume => {
                let value = match src_unit_code {
                    "CubicMeters" => value * 1000.0,
                    "Gallons" => value / GALLONS_PER_LITER,
                    "Liters" => value,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                };
                match dst_unit_code {
                    "CubicMeters" => value / 1000.0,
                    "Gallons" => value * GALLONS_PER_LITER,
                    "Liters" => value,
                    unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                }
            }
            UnitFamily::Time => {
                // let value = match src_unit_code {
                //     unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                // };
                // match dst_unit_code {
                //     unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                // }
                return Err(format!("Unexpected unit code {src_unit_code}").into());
            }
            UnitFamily::Power => {
                // let value = match src_unit_code {
                //     unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                // };
                // match dst_unit_code {
                //     unit_code => return Err(format!("Unexpected unit code {unit_code}").into()),
                // }
                return Err(format!("Unexpected unit code {src_unit_code}").into());
            }
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        test_data::SPEC_FILE_1,
        test_utils::{
            test_clone_derive, test_copy_derive, test_debug_derive, test_partial_eq_derive,
        },
    };

    #[test]
    fn test_error_kind_derived_impls() {
        let error_kind = ErrorKind::InvalidFileHeader;

        test_debug_derive(&error_kind);
        test_clone_derive(&error_kind);
        test_copy_derive(&error_kind);
    }

    fn call_err<T>() {
        let result = err::<T>(ErrorKind::InvalidFileHeader);

        assert!(result.is_err());

        let error = result.err().unwrap();

        assert_eq!("Unable to parse VSF: InvalidFileHeader", error.to_string());
    }

    #[test]
    fn test_err() {
        call_err::<()>();
        call_err::<String>();
        call_err::<DeviceTemplate>();
        call_err::<LocalizedText>();
        call_err::<PacketTemplate>();
        call_err::<PacketTemplateField>();
        call_err::<SpecificationFile>();
        call_err::<Unit>();
    }

    #[test]
    fn test_language_derived_impls() {
        let language = Language::En;

        test_debug_derive(&language);
        test_clone_derive(&language);
        test_copy_derive(&language);
        test_partial_eq_derive(&language);
    }

    #[test]
    fn test_text_index_derived_impls() {
        let text_index = TextIndex(0);

        test_debug_derive(&text_index);
        test_clone_derive(&text_index);
        test_copy_derive(&text_index);
    }

    #[test]
    fn test_localized_text_derived_impls() {
        let loc_text_index = LocalizedText {
            text_index_en: TextIndex(0),
            text_index_de: TextIndex(1),
            text_index_fr: TextIndex(2),
        };

        test_debug_derive(&loc_text_index);
    }

    #[test]
    fn test_localized_text_index_derived_impls() {
        let loc_text_index = LocalizedTextIndex(0);

        test_debug_derive(&loc_text_index);
        test_clone_derive(&loc_text_index);
        test_copy_derive(&loc_text_index);
    }

    #[test]
    fn test_unit_family_derived_impls() {
        let unit_family = UnitFamily::None;

        test_debug_derive(&unit_family);
        test_clone_derive(&unit_family);
        test_copy_derive(&unit_family);
        test_partial_eq_derive(&unit_family);
    }

    #[test]
    fn test_unit_derived_impls() {
        let unit = Unit {
            unit_id: UnitId(0),
            unit_family_id: UnitFamilyId(0),
            unit_code_text_index: TextIndex(0),
            unit_text_text_index: TextIndex(1),
        };

        test_debug_derive(&unit);
        test_clone_derive(&unit);
    }

    #[test]
    fn test_device_template_derived_impls() {
        let dt = DeviceTemplate {
            self_address: 0x0010,
            self_mask: 0xFFFF,
            peer_address: 0x0000,
            peer_mask: 0x0000,
            name_localized_text_index: LocalizedTextIndex(0),
        };

        test_debug_derive(&dt);
    }

    #[test]
    fn test_packet_template_derived_impls() {
        let pt = PacketTemplate {
            destination_address: 0x0010,
            destination_mask: 0xFFFF,
            source_address: 0x7E11,
            source_mask: 0xFFFF,
            command: 0x0100,
            fields: Vec::new(),
        };

        test_debug_derive(&pt);
        test_clone_derive(&pt);
    }

    #[test]
    fn test_type_derived_impls() {
        let typ = Type::Number;

        test_debug_derive(&typ);
        test_clone_derive(&typ);
        test_copy_derive(&typ);
        test_partial_eq_derive(&typ);
    }

    #[test]
    fn test_type_id_derived_impls() {
        let type_id = TypeId(0);

        test_debug_derive(&type_id);
        test_clone_derive(&type_id);
        test_copy_derive(&type_id);
        test_partial_eq_derive(&type_id);
    }

    #[test]
    fn test_packet_template_field_derived_impls() {
        let ptf = PacketTemplateField {
            id_text_index: TextIndex(0),
            name_localized_text_index: LocalizedTextIndex(0),
            unit_id: UnitId(0),
            precision: 0,
            type_id: TypeId(0),
            parts: Vec::new(),
        };

        test_debug_derive(&ptf);
        test_clone_derive(&ptf);
    }

    #[test]
    fn test_packet_template_field_part_derived_impls() {
        let ptfp = PacketTemplateFieldPart {
            offset: 0,
            bit_pos: 0,
            mask: 0xFF,
            is_signed: false,
            factor: 1,
        };

        test_debug_derive(&ptfp);
        test_clone_derive(&ptfp);
        test_partial_eq_derive(&ptfp);
    }

    #[test]
    fn test_specification_file_derived_impls() {
        let spec_file = SpecificationFile::new_default();

        test_debug_derive(&spec_file);
    }

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
        check_next_localized_text("Wärmemenge Monat", "Wärmemenge Monat", "Wärmemenge Monat");
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

        let mut check_next_unit = |unit_id, unit_family, unit_code, unit_text, ref_value: f64| {
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

            let ref_unit_code = match unit_family {
                UnitFamily::Temperature => Some(62),
                UnitFamily::Energy => Some(18),
                UnitFamily::VolumeFlow => Some(136),
                UnitFamily::Pressure => Some(55),
                UnitFamily::Volume => Some(82),
                UnitFamily::Time | UnitFamily::Power | UnitFamily::None => None,
            };

            if let Some(ref_unit_code) = ref_unit_code {
                let ref_unit = spec_file.unit_by_id(&UnitId(ref_unit_code));

                let value = spec_file
                    .convert_value(1.0, &ref_unit, &unit)
                    .expect("Should have converted");
                assert!((value - ref_value).abs() < 0.00001);

                let value = spec_file
                    .convert_value(1.0, &unit, &unit)
                    .expect("Should have converted");
                assert!((value - 1.0).abs() < 0.00001);
            }

            unit_index += 1;
        };

        assert_eq!(48, spec_file.units.len());
        check_next_unit(55, UnitFamily::Pressure, "Bars", " bar", 1.0);
        check_next_unit(20, UnitFamily::Energy, "Btus", " BTU", 3.412_128);
        check_next_unit(80, UnitFamily::Volume, "CubicMeters", " m³", 1.0 / 1000.0);
        check_next_unit(
            135,
            UnitFamily::VolumeFlow,
            "CubicMetersPerHour",
            " m³/h",
            1.0 / 1000.0,
        );
        check_next_unit(70, UnitFamily::None, "Days", " d", 0.0);
        check_next_unit(90, UnitFamily::None, "DegreesAngular", " °", 0.0);
        check_next_unit(62, UnitFamily::Temperature, "DegreesCelsius", " °C", 1.0);
        check_next_unit(
            64,
            UnitFamily::Temperature,
            "DegreesFahrenheit",
            " °F",
            33.8,
        );
        check_next_unit(63, UnitFamily::None, "DegreesKelvin", " K", 0.0);
        check_next_unit(1042, UnitFamily::Volume, "Gallons", " gal", 0.264_172);
        check_next_unit(
            1041,
            UnitFamily::VolumeFlow,
            "GallonsPerHour",
            " gal/h",
            0.264_172,
        );
        check_next_unit(
            1040,
            UnitFamily::VolumeFlow,
            "GallonsPerMinute",
            " gal/min",
            0.264_172 / 60.0,
        );
        check_next_unit(
            1035,
            UnitFamily::Energy,
            "GramsCO2Gas",
            " g CO₂ (Gas)",
            0.2536,
        );
        check_next_unit(
            1032,
            UnitFamily::Energy,
            "GramsCO2Oil",
            " g CO₂ (Oil)",
            0.568,
        );
        check_next_unit(133, UnitFamily::None, "Hectopascals", " hPa", 0.0);
        check_next_unit(27, UnitFamily::None, "Hertz", " Hz", 0.0);
        check_next_unit(71, UnitFamily::None, "Hours", " h", 0.0);
        check_next_unit(
            1030,
            UnitFamily::Energy,
            "KiloBtus",
            " MBTU",
            3.412_128 / 1000.0,
        );
        check_next_unit(
            1024,
            UnitFamily::None,
            "KiloWattHoursPerSquareMeterPerDay",
            " kWh/(m²*d)",
            0.0,
        );
        check_next_unit(
            1036,
            UnitFamily::Energy,
            "KilogramsCO2Gas",
            " kg CO₂ (Gas)",
            0.2536 / 1000.0,
        );
        check_next_unit(
            1033,
            UnitFamily::Energy,
            "KilogramsCO2Oil",
            " kg CO₂ (Oil)",
            0.568 / 1000.0,
        );
        check_next_unit(
            186,
            UnitFamily::None,
            "KilogramsPerCubicMeter",
            " kg/m³",
            0.0,
        );
        check_next_unit(44, UnitFamily::None, "KilogramsPerHour", " kg/h", 0.0);
        check_next_unit(
            19,
            UnitFamily::Energy,
            "KilowattHours",
            " kWh",
            1.0 / 1000.0,
        );
        check_next_unit(48, UnitFamily::None, "Kilowatts", " kW", 0.0);
        check_next_unit(82, UnitFamily::Volume, "Liters", " l", 1.0);
        check_next_unit(136, UnitFamily::VolumeFlow, "LitersPerHour", " l/h", 1.0);
        check_next_unit(
            88,
            UnitFamily::VolumeFlow,
            "LitersPerMinute",
            " l/min",
            1.0 / 60.0,
        );
        check_next_unit(
            1025,
            UnitFamily::None,
            "LitersPerSquareMeterPerDay",
            " l/(m²*d)",
            0.0,
        );
        check_next_unit(
            1031,
            UnitFamily::Energy,
            "MegaBtus",
            " MMBTU",
            3.412_128 / 1_000_000.0,
        );
        check_next_unit(
            146,
            UnitFamily::Energy,
            "MegawattHours",
            " MWh",
            1.0 / 1_000_000.0,
        );
        check_next_unit(74, UnitFamily::None, "MetersPerSecond", " m/s", 0.0);
        check_next_unit(1100, UnitFamily::None, "Microvolts", " µV", 0.0);
        check_next_unit(2, UnitFamily::None, "Milliamperes", " mA", 0.0);
        check_next_unit(159, UnitFamily::None, "Milliseconds", " ms", 0.0);
        check_next_unit(72, UnitFamily::None, "Minutes", " min", 0.0);
        check_next_unit(-1, UnitFamily::None, "None", "", 0.0);
        check_next_unit(4, UnitFamily::None, "Ohms", " \u{2126}", 0.0);
        check_next_unit(98, UnitFamily::None, "Percent", "%", 0.0);
        check_next_unit(
            56,
            UnitFamily::Pressure,
            "PoundsForcePerSquareInch",
            " psi",
            14.503_773_8,
        );
        check_next_unit(73, UnitFamily::None, "Seconds", " s", 0.0);
        check_next_unit(0, UnitFamily::None, "SquareMeters", " m²", 0.0);
        check_next_unit(
            1037,
            UnitFamily::Energy,
            "TonsCO2Gas",
            " t CO₂ (Gas)",
            0.2536 / 1_000_000.0,
        );
        check_next_unit(
            1034,
            UnitFamily::Energy,
            "TonsCO2Oil",
            " t CO₂ (Oil)",
            0.568 / 1_000_000.0,
        );
        check_next_unit(5, UnitFamily::None, "Volts", " V", 0.0);
        check_next_unit(18, UnitFamily::Energy, "WattHours", " Wh", 1.0);
        check_next_unit(47, UnitFamily::None, "Watts", " W", 0.0);
        check_next_unit(35, UnitFamily::None, "WattsPerSquareMeter", " W/m²", 0.0);

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
        // ErrorKind::InvalidFileHeader
        let bytes = &[
            0x18, 0x37, 0x18, 0x37, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!("Unable to parse VSF: InvalidFileHeader", error.to_string());

        // ErrorKind::InvalidFileHeaderTotalLength
        let bytes = &[
            0x18, 0x37, 0x18, 0x37, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidFileHeaderTotalLength",
            error.to_string()
        );

        // ErrorKind::InvalidFileHeaderChecksumA
        let bytes = &[
            0x19, 0x37, 0x18, 0x37, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidFileHeaderChecksumA",
            error.to_string()
        );

        // ErrorKind::InvalidFileHeaderChecksumB
        let bytes = &[
            0x18, 0x37, 0x19, 0x37, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidFileHeaderChecksumB",
            error.to_string()
        );

        // ErrorKind::InvalidFileHeaderDataVersion
        let bytes = &[
            0xa7, 0xb6, 0xa7, 0xb6, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidFileHeaderDataVersion",
            error.to_string()
        );

        // ErrorKind::InvalidFileHeaderSpecificationOffset
        let bytes = &[
            0x18, 0x37, 0x18, 0x37, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidFileHeaderSpecificationOffset",
            error.to_string()
        );

        // ErrorKind::InvalidSpecificationTextTable
        let bytes = &[
            0x91, 0xda, 0x91, 0xda, 0x3c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidSpecificationTextTable",
            error.to_string()
        );

        // ErrorKind::InvalidSpecificationLocalizedTextTable
        let bytes = &[
            0xbd, 0x17, 0xbd, 0x17, 0x3c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidSpecificationLocalizedTextTable",
            error.to_string()
        );

        // ErrorKind::InvalidSpecificationUnitTable
        let bytes = &[
            0x48, 0xd1, 0x48, 0xd1, 0x3c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidSpecificationUnitTable",
            error.to_string()
        );

        // ErrorKind::InvalidSpecificationDeviceTemplateTable
        let bytes = &[
            0x85, 0x34, 0x85, 0x34, 0x3c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidSpecificationDeviceTemplateTable",
            error.to_string()
        );

        // ErrorKind::InvalidSpecificationPacketTemplateTable
        let bytes = &[
            0x1e, 0xd1, 0x1e, 0xd1, 0x3c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x3c, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidSpecificationPacketTemplateTable",
            error.to_string()
        );

        // ErrorKind::InvalidTextStringOffset
        let bytes = &[
            0x53, 0x45, 0x53, 0x45, 0x4c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x4c, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidTextStringOffset",
            error.to_string()
        );

        // ErrorKind::InvalidTextContent
        let bytes = &[
            0x3a, 0x48, 0x3a, 0x48, 0x4d, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x4c, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc8,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!("Unable to parse VSF: InvalidTextContent", error.to_string());

        // ErrorKind::InvalidLocalizedTextTextIndexEn
        let bytes = &[
            0xcc, 0xa6, 0xcc, 0xa6, 0x58, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidLocalizedTextTextIndexEn",
            error.to_string()
        );

        // ErrorKind::InvalidLocalizedTextTextIndexDe
        let bytes = &[
            0xce, 0xd6, 0xce, 0xd6, 0x58, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidLocalizedTextTextIndexDe",
            error.to_string()
        );

        // ErrorKind::InvalidLocalizedTextTextIndexFr
        let bytes = &[
            0xfe, 0xb3, 0xfe, 0xb3, 0x58, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidLocalizedTextTextIndexFr",
            error.to_string()
        );

        // ErrorKind::InvalidUnitUnitFamilyId
        let bytes = &[
            0x6b, 0x82, 0x6b, 0x82, 0x68, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidUnitUnitFamilyId",
            error.to_string()
        );

        // ErrorKind::InvalidUnitUnitCodeTextIndex
        let bytes = &[
            0x4c, 0x3a, 0x4c, 0x3a, 0x68, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidUnitUnitCodeTextIndex",
            error.to_string()
        );

        // ErrorKind::InvalidUnitUnitTextTextIndex
        let bytes = &[
            0x7c, 0x5f, 0x7c, 0x5f, 0x68, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidUnitUnitTextTextIndex",
            error.to_string()
        );

        // ErrorKind::InvalidDeviceTemplateNameLocalizedTextIndex
        let bytes = &[
            0x2f, 0x38, 0x2f, 0x38, 0x74, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x48, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidDeviceTemplateNameLocalizedTextIndex",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldTable
        let bytes = &[
            0x17, 0xb3, 0x17, 0xb3, 0x88, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x5c, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x88, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldTable",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldIdTextIndex
        let bytes = &[
            0x26, 0xd3, 0x26, 0xd3, 0xa4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldIdTextIndex",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldNameLocalizedTextIndex
        let bytes = &[
            0xa9, 0xd4, 0xa9, 0xd4, 0xa4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldNameLocalizedTextIndex",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldUnitId
        let bytes = &[
            0x99, 0xee, 0x99, 0xee, 0xa4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldUnitId",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldTypeId
        let bytes = &[
            0xe8, 0x07, 0xe8, 0x07, 0xa4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldTypeId",
            error.to_string()
        );

        // ErrorKind::InvalidPacketTemplateFieldPartTable
        let bytes = &[
            0x56, 0xd2, 0x56, 0xd2, 0xa4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00,
            0x00, 0x00, 0x00, 0x54, 0x65, 0x78, 0x74, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
            0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xa4, 0x00,
            0x00, 0x00, 0x10, 0x00, 0xff, 0xff, 0x11, 0x7e, 0xff, 0xff, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0xd5, 0xaf, 0x34, 0x01, 0x02, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00,
        ];

        let error = SpecificationFile::from_bytes(bytes).unwrap_err();

        assert_eq!(
            "Unable to parse VSF: InvalidPacketTemplateFieldPartTable",
            error.to_string()
        );

        // valid test fixture
        let spec_file = SpecificationFile::from_bytes(SPEC_FILE_1).unwrap();

        check_spec_file_fixture(&spec_file);
    }

    #[test]
    fn test_new_default() {
        let _spec_file = SpecificationFile::new_default();
    }

    #[test]
    fn test_unit_family_by_id() {
        let spec_file = SpecificationFile::new_default();

        assert_eq!(
            UnitFamily::None,
            spec_file.unit_family_by_id(&UnitFamilyId(-1))
        );
        assert_eq!(
            UnitFamily::Temperature,
            spec_file.unit_family_by_id(&UnitFamilyId(0))
        );
        assert_eq!(
            UnitFamily::Energy,
            spec_file.unit_family_by_id(&UnitFamilyId(1))
        );
        assert_eq!(
            UnitFamily::VolumeFlow,
            spec_file.unit_family_by_id(&UnitFamilyId(2))
        );
        assert_eq!(
            UnitFamily::Pressure,
            spec_file.unit_family_by_id(&UnitFamilyId(3))
        );
        assert_eq!(
            UnitFamily::Volume,
            spec_file.unit_family_by_id(&UnitFamilyId(4))
        );
        assert_eq!(
            UnitFamily::Time,
            spec_file.unit_family_by_id(&UnitFamilyId(5))
        );
        assert_eq!(
            UnitFamily::Power,
            spec_file.unit_family_by_id(&UnitFamilyId(6))
        );
    }

    #[test]
    #[should_panic(expected = "Unsupported unit family ID UnitFamilyId(-2)")]
    fn test_unit_family_by_id_panic() {
        let spec_file = SpecificationFile::new_default();

        spec_file.unit_family_by_id(&UnitFamilyId(-2));
    }

    #[test]
    fn test_unit_by_unit_code() {
        let spec_file = SpecificationFile::new_default();

        let unit = spec_file
            .unit_by_unit_code("DegreesCelsius")
            .expect("Unit should exist");

        assert_eq!(UnitId(62), unit.unit_id);
    }

    #[test]
    fn test_type_by_id() {
        let spec_file = SpecificationFile::new_default();

        assert_eq!(Type::Number, spec_file.type_by_id(&TypeId(1)));
        assert_eq!(Type::Time, spec_file.type_by_id(&TypeId(3)));
        assert_eq!(Type::WeekTime, spec_file.type_by_id(&TypeId(4)));
        assert_eq!(Type::DateTime, spec_file.type_by_id(&TypeId(5)));
    }

    #[test]
    #[should_panic(expected = "Unsupported type ID TypeId(-1)")]
    fn test_type_by_id_panic() {
        let spec_file = SpecificationFile::new_default();

        spec_file.type_by_id(&TypeId(-1));
    }

    #[test]
    fn test_convert_value() {
        let spec_file = SpecificationFile::new_default();

        let assert_err_with_units = |expected_error, src_unit, dst_unit| {
            let error = spec_file
                .convert_value(0.0, src_unit, dst_unit)
                .err()
                .unwrap();

            assert_eq!(expected_error, error.to_string());
        };

        let assert_err_with_codes =
            |expected_error, unit_family_id, src_unit_code, dst_unit_code| {
                let mut src_unit = spec_file
                    .unit_by_unit_code(src_unit_code)
                    .expect("Source unit should exist")
                    .clone();
                src_unit.unit_family_id = UnitFamilyId(unit_family_id);

                let mut dst_unit = spec_file
                    .unit_by_unit_code(dst_unit_code)
                    .expect("Destination unit should exist")
                    .clone();
                dst_unit.unit_family_id = UnitFamilyId(unit_family_id);

                let error = spec_file
                    .convert_value(0.0, &src_unit, &dst_unit)
                    .err()
                    .unwrap();

                assert_eq!(expected_error, error.to_string());
            };

        assert_err_with_units(
            "Unit families differ",
            spec_file
                .unit_by_unit_code("DegreesCelsius")
                .expect("Source unit should exists"),
            spec_file
                .unit_by_unit_code("Btus")
                .expect("Destination unit should exist"),
        );

        assert_err_with_codes(
            "Cannot convert values with UnitFamily::None",
            -1,
            "DegreesCelsius",
            "DegreesCelsius",
        );

        assert_err_with_codes("Unexpected unit code Btus", 0, "Btus", "DegreesCelsius");
        assert_err_with_codes("Unexpected unit code Btus", 0, "DegreesCelsius", "Btus");

        assert_err_with_codes("Unexpected unit code Bars", 1, "Bars", "Btus");
        assert_err_with_codes("Unexpected unit code Bars", 1, "Btus", "Bars");

        assert_err_with_codes("Unexpected unit code Btus", 2, "Btus", "LitersPerHour");
        assert_err_with_codes("Unexpected unit code Btus", 2, "LitersPerHour", "Btus");

        assert_err_with_codes("Unexpected unit code Btus", 3, "Btus", "Bars");
        assert_err_with_codes("Unexpected unit code Btus", 3, "Bars", "Btus");

        assert_err_with_codes("Unexpected unit code Btus", 4, "Btus", "Liters");
        assert_err_with_codes("Unexpected unit code Btus", 4, "Liters", "Btus");

        assert_err_with_codes("Unexpected unit code Btus", 5, "Btus", "Btus");

        assert_err_with_codes("Unexpected unit code Btus", 6, "Btus", "Btus");
    }
}
