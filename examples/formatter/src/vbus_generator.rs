use resol_vbus::{chrono::Local, RecordingWriter};

use crate::{
    app_error::Result, config::Config, timestamp_file_writer::TimestampFileWriter,
    timestamp_interval::TimestampInterval,
};

pub fn generate(config: &mut Config) -> Result<()> {
    let dsr = &mut config.data_set_reader;
    let interval_duration = config.sieve_interval;
    let ttl_duration = config.ttl;
    let topology_data_set = config.topology_data_set;
    let pattern = config.output_pattern.unwrap_or("Output.vbus");

    let output_writer = TimestampFileWriter::new(pattern.to_owned());

    let mut cumultative_data_set = topology_data_set.clone();

    let mut timestamp_interval = TimestampInterval::new(interval_duration);

    let mut output = RecordingWriter::new(output_writer);

    while let Some(data_set) = dsr.read_data_set()? {
        let timestamp = data_set.timestamp;
        let local_timestamp = timestamp.with_timezone(&Local);

        let new_interval = timestamp_interval.is_new_interval(&local_timestamp);

        output.get_mut().set_timestamp(timestamp)?;

        cumultative_data_set.add_data_set(data_set);
        if let Some(duration) = ttl_duration {
            cumultative_data_set.clear_packets_older_than(timestamp - duration);
        }
        cumultative_data_set.timestamp = timestamp;

        if new_interval {
            output.write_data_set(&cumultative_data_set)?;
            cumultative_data_set.remove_all_data();
        }
    }

    Ok(())
}
