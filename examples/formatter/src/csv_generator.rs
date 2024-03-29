use std::io::Write;

use resol_vbus::chrono::Local;

use crate::{
    app_error::Result, config::Config, field_iterator::*,
    timestamp_file_writer::TimestampFileWriter, timestamp_interval::TimestampInterval,
};

pub fn convert_to_text_data(config: &mut Config<'_>) -> Result<()> {
    let dsr = &mut config.data_set_reader;
    let interval_duration = config.sieve_interval;
    let ttl_duration = config.ttl;
    let topology_data_set = config.topology_data_set;
    let spec = config.specification;
    let pattern = config.output_pattern.unwrap_or("Output.csv");
    let local_timezone = config.local_timezone;

    let mut output_writer = TimestampFileWriter::new(pattern.to_owned(), local_timezone);

    let mut cumultative_data_set = topology_data_set.clone();

    let mut timestamp_interval = TimestampInterval::new(interval_duration);

    let field_iterator = AllFieldsIterator::new(spec);

    let output = &mut output_writer;

    let sep = "\t";
    let eol = "\n";

    while let Some((data_set, comments)) = dsr.read_data_set_and_comments()? {
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

                write!(output, "{}", sep)?;

                if new_packet_spec {
                    current_packet_id = Some(field.packet_spec().packet_id.clone());
                    write!(output, "{}", field.packet_spec().name)?;
                }
            }

            write!(output, "{}", eol)?;

            // Row 3: Field names
            write!(output, "Date / Time")?;

            for field in field_iterator.fields_in_data_set(&cumultative_data_set) {
                write!(output, "{}{}", sep, field.field_spec().name)?;
            }

            // for data in cumultative_data_set.as_data_slice() {
            //     write!(output, "\t{}", data.id_string())?;
            // }

            write!(output, "{}", eol)?;
        }

        if new_interval {
            // write!(output, "{:?} {:?} {:?} {:?} {:?} {:?}  ", timestamp.timestamp(), local_timestamp.timestamp(), timestamp.naive_utc().timestamp(), timestamp.naive_local().timestamp(), local_timestamp.naive_utc().timestamp(), local_timestamp.naive_local().timestamp());

            if local_timezone {
                write!(output, "{}", local_timestamp.to_rfc3339())?;
            } else {
                write!(output, "{}", timestamp.to_rfc3339())?;
            }

            for field in field_iterator.fields_in_data_set(&cumultative_data_set) {
                write!(output, "{}{}", sep, field.fmt_raw_value(false))?;
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

            for comment in comments {
                let comment_str = match std::str::from_utf8(comment.comment()) {
                    Ok(comment_str) => comment_str,
                    Err(_) => "<non UTF-8 comment>",
                };

                write!(output, "{}{}", sep, comment_str)?;
            }

            write!(output, "{}", eol)?;
        }
    }

    Ok(())
}
