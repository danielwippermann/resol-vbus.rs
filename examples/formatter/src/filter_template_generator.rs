use resol_vbus::*;

use field_iterator::{AllFieldsIterator, FieldIterator};


pub fn print_filter_template(data_set: &DataSet, spec: &Specification) {
    let field_iterator = AllFieldsIterator::new(spec);

    println!("use resol_vbus::{{PacketId, PacketFieldId, Specification}};");
    println!("");
    println!("use field_iterator::FilteredFieldIterator;");
    println!("");
    println!("");
    println!("pub fn create_xxx_filtered_field_iterator<'a>(spec: &'a Specification) -> FilteredFieldIterator<'a> {{");
    println!("    FilteredFieldIterator::new(spec, vec![");

    let mut last_packet_id = None;

    for field in field_iterator.fields_in_data_set(data_set) {
        let packet_id = field.packet_id();

        let PacketId(channel, destination_address, source_address, command) = packet_id;

        if last_packet_id != Some(packet_id) {
            last_packet_id = Some(packet_id);

            println!("");
            println!("        //-----------------------------------------------------------------");
            println!("        // {}", field.data().id_string());
            println!("        // {}", field.packet_spec().name);
            println!("        //-----------------------------------------------------------------");
        }

        println!("        PacketFieldId(PacketId(0x{:02X}, 0x{:04X}, 0x{:04X}, 0x{:04X}), \"{}\"),  // {}", channel, destination_address, source_address, command, field.field_spec().field_id, field.field_spec().name);
    }

    println!("    ])");
    println!("}}");
}
