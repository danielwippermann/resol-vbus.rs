use std::io::Write;

// use resol_vbus::chrono::Local;

use super::{
    dtpv_filtered_field_iterator::create_dtpv_filtered_field_iterator,
    field_iterator::FieldIterator,
    timestamp_file_writer::TimestampFileWriter,
    // timestamp_interval::TimestampInterval,
    Config,
    Result,
};

pub fn generate(config: &mut Config<'_>) -> Result<()> {
    let dsr = &mut config.data_set_reader;
    // let interval_duration = config.sieve_interval;
    let spec = config.specification;
    let pattern = config.output_pattern.unwrap_or("Output.txt");
    let local_timezone = config.local_timezone;

    let mut output_writer = TimestampFileWriter::new(pattern.to_owned(), local_timezone);

    // let mut cumultative_data_set = topology_data_set.clone();

    // let mut timestamp_interval = TimestampInterval::new(interval_duration);

    // let field_iterator = AllFieldsIterator::new(spec);

    let field_iterator = create_dtpv_filtered_field_iterator(spec);

    let output = &mut output_writer;

    let mut last_irradiation_state = false;
    let mut last_change_timestamp = None;
    let mut histogram = [0; 5];

    while let Some(data_set) = dsr.read_data_set()? {
        let timestamp = data_set.timestamp;
        // let local_timestamp = timestamp.with_timezone(&Local);

        // let new_interval = timestamp_interval.is_new_interval(&local_timestamp);

        // let is_new_file = output.set_timestamp(timestamp)?;

        let mut it = field_iterator.fields_in_data_set(&data_set);
        let field1 = it.next().unwrap();
        let field2 = it.next().unwrap();

        // dbg!(v1, v2);
        let (diff, irradiation_state) =
            if let (Some(v1), Some(v2)) = (field1.raw_value_f64(), field2.raw_value_f64()) {
                (Some(v1 - v2), v1 > 0.0 && v1 >= 3030.0)
            } else {
                (None, false)
            };

        if last_irradiation_state != irradiation_state {
            last_irradiation_state = irradiation_state;

            let time_diff = if let Some(last_change_timestamp) = last_change_timestamp {
                let seconds = timestamp
                    .signed_duration_since(&last_change_timestamp)
                    .num_seconds();

                if !irradiation_state {
                    if seconds < 10 {
                        histogram[0] += 1;
                    } else if seconds < 60 {
                        histogram[1] += 1;
                    } else if seconds < 600 {
                        histogram[2] += 1;
                    } else {
                        histogram[3] += 1;
                    }
                }

                Some(seconds)
            } else {
                None
            };

            last_change_timestamp = Some(timestamp.clone());

            writeln!(
                output,
                "{}\t{}\t{}\t{:?}\t{:?}",
                timestamp.to_rfc3339(),
                field1.fmt_raw_value(false),
                field2.fmt_raw_value(false),
                diff,
                time_diff
            )?;
        }
    }

    println!("{:?}", histogram);

    Ok(())
}
