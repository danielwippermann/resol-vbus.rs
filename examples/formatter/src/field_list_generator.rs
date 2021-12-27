use resol_vbus::*;

use crate::{
    config::Config,
    field_iterator::{AllFieldsIterator, FieldIterator},
};

pub fn print_data_set_fields(config: &mut Config<'_>) {
    let mut last_packet_id = None;

    let field_iterator = AllFieldsIterator::new(config.specification);

    for field in field_iterator.fields_in_data_set(config.topology_data_set) {
        let PacketFieldId(packet_id, field_id) = field.packet_field_id();

        if last_packet_id != Some(packet_id) {
            last_packet_id = Some(packet_id);

            println!();
            println!("// ----------------------------------------------------------------------");
            println!("// -- {:64} --", packet_id.packet_id_string());
            println!("// -- {:64} --", field.packet_spec().name);
            println!("// ----------------------------------------------------------------------");
        }

        println!(
            "PacketFieldId(PacketId(0x{:02X}, 0x{:04X}, 0x{:04X}, 0x{:04X}), {:?}),  // {}",
            packet_id.0,
            packet_id.1,
            packet_id.2,
            packet_id.3,
            field_id,
            field.field_spec().name
        );
    }
}
