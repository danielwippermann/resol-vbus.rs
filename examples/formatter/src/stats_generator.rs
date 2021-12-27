use resol_vbus::*;

use crate::{app_error::Result, config::Config};

pub fn print_stats(config: &mut Config<'_>) -> Result<()> {
    let dsr = &mut config.data_set_reader;

    let mut min_timestamp = None;
    let mut max_timestamp = None;
    let mut total_data_count = 0;
    let mut total_data_set_count = 0;

    while let Some(data_set) = dsr.read_data_set()? {
        let timestamp = data_set.timestamp;

        if min_timestamp.is_none() || min_timestamp.unwrap() > timestamp {
            min_timestamp = Some(timestamp);
        }
        if max_timestamp.is_none() || max_timestamp.unwrap() < timestamp {
            max_timestamp = Some(timestamp);
        }

        total_data_set_count += 1;
        total_data_count += data_set.as_data_slice().len();
    }

    if total_data_set_count > 0 {
        println!("Min. timestamp: {}", min_timestamp.unwrap().to_rfc3339());
        println!("Max. timestamp: {}", max_timestamp.unwrap().to_rfc3339());
        println!("Data set count: {}", total_data_set_count);
        println!("Data count: {}", total_data_count);
        println!("Data IDs:");
        for data in config.topology_data_set.as_data_slice() {
            let description = match *data {
                Data::Packet(ref packet) => config
                    .specification
                    .get_packet_spec_by_id(packet.packet_id())
                    .name
                    .to_owned(),
                _ => "".to_owned(),
            };
            println!("- {}: {}", data.id_string(), description);
        }
    } else {
        println!("No data set found");
    }

    Ok(())
}
