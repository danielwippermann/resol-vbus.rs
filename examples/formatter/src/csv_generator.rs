use std::io::Write;

use resol_vbus::chrono::{Local};

use app_error::Result;
use config::Config;
use field_iterator::*;
use timestamp_file_writer::TimestampFileWriter;
use timestamp_interval::TimestampInterval;


pub fn convert_to_text_data(config: &mut Config) -> Result<()> {
    let dsr = &mut config.data_set_reader;
    let interval_duration = config.sieve_interval;
    let ttl_duration = config.ttl;
    let topology_data_set = config.topology_data_set;
    let spec = config.specification;
    let pattern = config.output_pattern.unwrap_or("Output.csv");

    let mut output_writer = TimestampFileWriter::new(pattern.to_owned());

    let mut cumultative_data_set = topology_data_set.clone();

    let mut timestamp_interval = TimestampInterval::new(interval_duration);

    let field_iterator = AllFieldsIterator::new(spec);

    let output = &mut output_writer;

    while let Some(data_set) = dsr.read_data_set()? {
        let timestamp = data_set.timestamp;
        let local_timestamp = timestamp.with_timezone(&Local);

        let new_interval = timestamp_interval.is_new_interval(&local_timestamp);

        let is_new_file = output.set_timestamp(timestamp)?;

        // let this_packet_ids: Vec<_> = data_set.iter().filter(|data| data.is_packet()).map(|data| data.as_packet().packet_id()).collect();

        // cumultative_data_set.clear_all_packets();
        cumultative_data_set.add_data_set(data_set);
        if let Some(duration) = ttl_duration {
            cumultative_data_set.clear_packets_older_than(timestamp - duration);
        }
        cumultative_data_set.timestamp = timestamp;

        if is_new_file {
            println!("Generating \"{}\"...", output.filename().unwrap());

            // // Row 0: Packet IDs
            // let mut current_packet_id: Option<String> = None;
            //
            // for field in fields_in_data_set(&spec, &cumultative_data_set) {
            //     let new_packet_spec = match current_packet_id {
            //         Some(ref packet_id) => field.packet_spec().packet_id != *packet_id,
            //         None => true,
            //     };
            //
            //     write!(output, "\t")?;
            //
            //     if new_packet_spec {
            //         current_packet_id = Some(field.packet_spec().packet_id.clone());
            //         write!(output, "{}", field.packet_spec().packet_id)?;
            //     }
            // }
            //
            // write!(output, "\n")?;
            //
            // // Row 2: Field IDs
            // write!(output, "Date / Time")?;
            //
            // for field in fields_in_data_set(&spec, &cumultative_data_set) {
            //     write!(output, "\t{}", field.field_spec().field_id)?;
            // }
            //
            // write!(output, "\n")?;

            // Row 2: Packet names
            let mut current_packet_id = None;

            for field in field_iterator.fields_in_data_set(&cumultative_data_set) {
                let new_packet_spec = match current_packet_id {
                    Some(ref packet_id) => field.packet_spec().packet_id != *packet_id,
                    None => true,
                };

                write!(output, "\t")?;

                if new_packet_spec {
                    current_packet_id = Some(field.packet_spec().packet_id.clone());
                    write!(output, "{}", field.packet_spec().name)?;
                }
            }

            write!(output, "\n")?;

            // Row 3: Field names
            write!(output, "Date / Time")?;

            for field in field_iterator.fields_in_data_set(&cumultative_data_set) {
                write!(output, "\t{}", field.field_spec().name)?;
            }

            // for data in cumultative_data_set.as_data_slice() {
            //     write!(output, "\t{}", data.id_string())?;
            // }

            write!(output, "\n")?;
        }

        if new_interval {
            // write!(output, "{:?} {:?} {:?} {:?} {:?} {:?}  ", timestamp.timestamp(), local_timestamp.timestamp(), timestamp.naive_utc().timestamp(), timestamp.naive_local().timestamp(), local_timestamp.naive_utc().timestamp(), local_timestamp.naive_local().timestamp());

            write!(output, "{}", local_timestamp.to_rfc3339())?;

            for field in field_iterator.fields_in_data_set(&cumultative_data_set) {
                write!(output, "\t{}", field.fmt_raw_value(false))?;
            }

            // for data in cumultative_data_set.as_data_slice() {
            //     let is_current_packet = match *data {
            //         Data::Packet(ref packet) => this_packet_ids.contains(&packet.packet_id()),
            //         _ => false,
            //     };
            //     if is_current_packet {
            //         write!(output, "\t{}", "X")?;
            //     } else {
            //         write!(output, "\t")?;
            //     }
            // }

            write!(output, "\n")?;
        }
    }

    Ok(())
}
