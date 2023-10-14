#![deny(warnings)]
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(rust_2021_compatibility)]

use std::{fs::File, io::Read};

use clap::{Arg, Command};
use log::trace;
use resol_vbus::{
    chrono::{DateTime, Duration, Local, Utc},
    *,
};

mod app_error;
mod config;
mod csv_generator;
mod data_set_reader;
mod field_iterator;
mod field_list_generator;
mod filter_template_generator;
mod packet_list_generator;
mod simple_json_generator;
mod stats_generator;
mod timestamp_file_writer;
mod timestamp_interval;
mod vbus_generator;

use crate::{
    app_error::{Error, Result},
    config::Config,
    csv_generator::convert_to_text_data,
    field_list_generator::print_data_set_fields,
    filter_template_generator::print_filter_template,
    packet_list_generator::print_data_set_packets,
    stats_generator::print_stats,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum SourceType {
    Type44,
    Type88,
}

fn read_topology_data_set(
    input_filenames: Vec<String>,
    min_timestamp: Option<DateTime<Utc>>,
    max_timestamp: Option<DateTime<Utc>>,
) -> Result<(SourceType, DataSet)> {
    let flr = FileListReader::new(input_filenames.clone());
    let mut rr = RecordingReader::new(flr);

    let record = rr.read_record()?;
    if record.len() < 2 {
        Err(Error::from("No valid record found in file"))
    } else {
        match record[1] {
            0x44 => {
                let flr = FileListReader::new(input_filenames);
                let mut rr = RecordingReader::new(flr);
                rr.set_min_max_timestamps(min_timestamp, max_timestamp);

                let topology_data_set = rr.read_topology_data_set()?;

                Ok((SourceType::Type44, topology_data_set))
            }
            0x88 => {
                let flr = FileListReader::new(input_filenames);
                let mut rr = LiveDataRecordingReader::new(flr);
                rr.set_min_max_timestamps(min_timestamp, max_timestamp);

                let topology_data_set = rr.read_topology_data_set()?;

                Ok((SourceType::Type88, topology_data_set))
            }
            _ => Err(Error::from(format!(
                "Unexpected record type 0x{:02X}",
                record[1]
            ))),
        }
    }
}

fn process_data_set_stream(typ: &str, config: &mut Config<'_>) -> Result<bool> {
    let mut handled = true;
    match typ {
        "stats" => print_stats(config)?,
        "packets" => print_data_set_packets(config),
        "fields" => print_data_set_fields(config),
        "filter-template" => print_filter_template(config),
        "csv" => convert_to_text_data(config)?,
        "simple-json" => simple_json_generator::generate(config)?,
        "vbus" => vbus_generator::generate(config)?,
        _ => handled = false,
    }

    Ok(handled)
}

fn run() -> Result<()> {
    env_logger::init();

    let start = Local::now();

    let matches = Command::new("VBus-Formatter")
        .version("1.0")
        .author("Daniel Wippermann <Daniel.Wippermann@gmail.com>")
        .about("Formats recorded VBus data")
        .arg(
            Arg::new("type")
                .help("Sets the output type")
                .required(true)
                .num_args(1)
                .value_name("TYPE")
                .value_parser([
                    "raw-stats",
                    "stats",
                    "packets",
                    "fields",
                    "filter-template",
                    "csv",
                    "simple-json",
                    "vbus",
                ]),
        )
        .arg(
            Arg::new("sieve_interval")
                .help("Sieves input data and removes multiple data sets within the same interval")
                .long("sieve-interval")
                .num_args(1)
                .value_name("SECONDS"),
        )
        .arg(
            Arg::new("ttl")
                .help("Remove data from data sets if it was not updated for this amount of time")
                .long("ttl")
                .num_args(1)
                .value_name("SECONDS"),
        )
        .arg(
            Arg::new("min_timestamp")
                .help("Ignore data sets before this point in time")
                .long("min-timestamp")
                .num_args(1)
                .value_name("DATETIME"),
        )
        .arg(
            Arg::new("max_timestamp")
                .help("Ignore data sets after this point in time")
                .long("max-timestamp")
                .num_args(1)
                .value_name("DATETIME"),
        )
        .arg(
            Arg::new("vsf_filename")
                .help("Location of the VSF file")
                .long("vsf")
                .num_args(1)
                .value_name("FILENAME"),
        )
        .arg(
            Arg::new("language")
                .help("Language")
                .long("language")
                .num_args(1)
                .value_name("LANGUAGE")
                .value_parser([
                    "en",
                    "de",
                    "fr",
                ]),
        )
        .arg(
            Arg::new("output_pattern")
                .help("Output filename pattern, optionally containing strftime placeholders")
                .long("output")
                .num_args(1)
                .value_name("PATTERN"),
        )
        .arg(
            Arg::new("local_timezone")
                .help("Use local timezone in text formatters and filename generation")
                .long("local-timezone"),
        )
        .arg(
            Arg::new("INPUT")
                .help("Sets the input files to use")
                .required(true)
                .num_args(1..),
        )
        .get_matches();

    let typ = matches.get_one::<String>("type").unwrap();

    let sieve_interval = if let Some(arg) = matches.get_one::<i64>("sieve_interval") {
        let seconds = *arg;
        if seconds > 0 {
            Some(Duration::seconds(seconds))
        } else {
            None
        }
    } else {
        None
    };

    let ttl_duration = if let Some(arg) = matches.get_one::<i64>("ttl") {
        let seconds = *arg;
        Some(Duration::seconds(seconds))
    } else {
        None
    };

    let min_timestamp = if let Some(arg) = matches.get_one::<String>("min_timestamp") {
        Some(arg.parse()?)
    } else {
        None
    };

    let max_timestamp = if let Some(arg) = matches.get_one::<String>("max_timestamp") {
        Some(arg.parse()?)
    } else {
        None
    };

    let vsf_filename = matches.get_one::<String>("vsf_filename");

    let language = match matches.get_one::<String>("language").map(|s| s.as_str()) {
        None => Language::De,
        Some("en") => Language::En,
        Some("de") => Language::De,
        Some("fr") => Language::Fr,
        Some(lang) => panic!("Unexpected language {}", lang),
    };

    let output_pattern = matches.get_one::<String>("output_pattern").map(|s| s.as_str());

    let local_timezone = matches.contains_id("local_timezone");

    let input_filenames = matches
        .get_many::<String>("INPUT")
        .unwrap()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let spec_file = match vsf_filename {
        Some(filename) => {
            let mut f = File::open(filename)?;

            let mut buf = Vec::new();
            let size = f.read_to_end(&mut buf)?;

            SpecificationFile::from_bytes(&buf[0..size])?
        }
        None => SpecificationFile::new_default(),
    };

    let spec = Specification::from_file(spec_file, language);

    let (source_type, topology_data_set) =
        read_topology_data_set(input_filenames.clone(), min_timestamp, max_timestamp)?;

    let flr = FileListReader::new(input_filenames);

    if source_type == SourceType::Type44 {
        let mut rr = RecordingReader::new(flr);
        rr.set_min_max_timestamps(min_timestamp, max_timestamp);

        let mut config = Config {
            sieve_interval,
            ttl: ttl_duration,
            min_timestamp,
            max_timestamp,
            language,
            specification: &spec,
            topology_data_set: &topology_data_set,
            data_set_reader: &mut rr,
            output_pattern,
            local_timezone,
        };

        if process_data_set_stream(typ, &mut config)? {
            // nop
        } else {
            panic!("Unsupported output type {} for Type 0x44 stream", typ);
        }
    } else if source_type == SourceType::Type88 {
        let mut ldrr = LiveDataRecordingReader::new(flr);
        ldrr.set_min_max_timestamps(min_timestamp, max_timestamp);

        if typ == "raw-stats" {
            let stats = ldrr.read_to_stats()?;
            println!("{:?}", stats);
        } else {
            let mut config = Config {
                sieve_interval,
                ttl: ttl_duration,
                min_timestamp,
                max_timestamp,
                language,
                specification: &spec,
                topology_data_set: &topology_data_set,
                data_set_reader: &mut ldrr,
                output_pattern,
                local_timezone,
            };

            if process_data_set_stream(typ, &mut config)? {
                // nop
            } else {
                panic!("Unsupported output type {} for Type 0x88 stream", typ);
            }
        }
    } else {
        panic!("Unsupported source record type {:?}", source_type);
    }

    // analyze_kioto_fwr_runtime(input_filenames, sieve_interval, ttl_duration, topology_data_set, &spec)?;

    trace!("runtime: {:?}", Local::now().signed_duration_since(start));

    Ok(())
}

fn main() {
    run().unwrap();
}
