extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate resol_vbus;


use std::fs::{File};
use std::io::Read;

use clap::{Arg, App};

use resol_vbus::*;
use resol_vbus::chrono::{DateTime, Duration, Local, UTC};


mod app_error;
use app_error::{AppError, Result};

mod field_iterator;

mod timestamp_interval;

mod timestamp_file_writer;

mod data_set_reader;

mod config;
use config::Config;

mod packet_list_generator;
use packet_list_generator::print_data_set_packets;

mod field_list_generator;
use field_list_generator::print_data_set_fields;

mod filter_template_generator;
use filter_template_generator::print_filter_template;

mod csv_generator;
use csv_generator::convert_to_text_data;


#[derive(Debug, PartialEq)]
enum SourceType {
    Type44,
    Type88,
}


fn read_topology_data_set(input_filenames: Vec<String>, min_timestamp: Option<DateTime<UTC>>, max_timestamp: Option<DateTime<UTC>>) -> Result<(SourceType, DataSet)> {
    let flr = FileListReader::new(input_filenames.clone());
    let mut rr = RecordingReader::new(flr);

    let record = rr.read_record()?;
    if record.len() < 2 {
        Err(AppError::from("No valid record found in file"))
    } else {
        match record [1] {
            0x44 => {
                let flr = FileListReader::new(input_filenames.clone());
                let mut rr = RecordingReader::new(flr);
                rr.set_min_max_timestamps(min_timestamp, max_timestamp);

                let topology_data_set = rr.read_topology_data_set()?;

                Ok((SourceType::Type44, topology_data_set))
            },
            0x88 => {
                let flr = FileListReader::new(input_filenames.clone());
                let mut rr = LiveDataRecordingReader::new(flr);
                rr.set_min_max_timestamps(min_timestamp, max_timestamp);

                let topology_data_set = rr.read_topology_data_set()?;

                Ok((SourceType::Type88, topology_data_set))
            },
            _ => {
                Err(AppError::from(format!("Unexpected record type 0x{:02X}", record [1])))
            }
        }
    }
}


fn process_topology_data_set(typ: &str, data_set: &DataSet, spec: &Specification) -> Result<bool> {
    let mut handled = true;
    match typ {
        "packets" => print_data_set_packets(data_set, spec),
        "fields" => print_data_set_fields(data_set, spec),
        "filter-template" => print_filter_template(data_set, spec),
        _ => handled = false,
    }

    Ok(handled)
}


fn process_data_set_stream(typ: &str, config: &mut Config) -> Result<bool> {
    let mut handled = true;
    match typ {
        "csv" => convert_to_text_data(config)?,
        _ => handled = false,
    }

    Ok(handled)
}


fn run() -> Result<()> {
    env_logger::init().unwrap();

    let start = Local::now();

    let matches = App::new("VBus-Formatter")
        .version("1.0")
        .author("Daniel Wippermann <Daniel.Wippermann@gmail.com>")
        .about("Formats recorded VBus data")
        .arg(Arg::with_name("type")
            .help("Sets the output type")
            .required(true)
            .takes_value(true)
            .value_name("TYPE")
            .possible_values(&[
                "packets",
                "fields",
                "filter-template",
                "csv",
            ]))
        .arg(Arg::with_name("sieve_interval")
            .help("Sieves input data and removes multiple data sets within the same interval")
            .long("sieve-interval")
            .takes_value(true)
            .value_name("SECONDS"))
        .arg(Arg::with_name("ttl")
            .help("Remove data from data sets if it was not updated for this amount of time")
            .long("ttl")
            .takes_value(true)
            .value_name("SECONDS"))
        .arg(Arg::with_name("min_timestamp")
            .help("Ignore data sets before this point in time")
            .long("min-timestamp")
            .takes_value(true)
            .value_name("DATETIME"))
        .arg(Arg::with_name("max_timestamp")
            .help("Ignore data sets after this point in time")
            .long("max-timestamp")
            .takes_value(true)
            .value_name("DATETIME"))
        .arg(Arg::with_name("vsf_filename")
            .help("Location of the VSF file")
            .long("vsf")
            .takes_value(true)
            .value_name("FILENAME"))
        .arg(Arg::with_name("language")
            .help("Language")
            .long("language")
            .takes_value(true)
            .value_name("LANGUAGE"))
        .arg(Arg::with_name("output_pattern")
            .help("Output filename pattern, optionally containing strftime placeholders")
            .long("output")
            .takes_value(true)
            .value_name("PATTERN"))
        .arg(Arg::with_name("INPUT")
            .help("Sets the input files to use")
            .required(true)
            .multiple(true))
        .get_matches();

    let typ = matches.value_of("type").unwrap();

    let sieve_interval = if let Some(arg) = matches.value_of("sieve_interval") {
        let seconds = arg.parse()?;
        if seconds > 0 {
            Some(Duration::seconds(seconds))
        } else {
            None
        }
    } else {
        None
    };

    let ttl_duration = if let Some(arg) = matches.value_of("ttl") {
        let seconds = arg.parse()?;
        Some(Duration::seconds(seconds))
    } else {
        None
    };

    let min_timestamp = if let Some(arg) = matches.value_of("min_timestamp") {
        Some(arg.parse()?)
    } else {
        None
    };

    let max_timestamp = if let Some(arg) = matches.value_of("max_timestamp") {
        Some(arg.parse()?)
    } else {
        None
    };

    let vsf_filename = matches.value_of("vsf_filename");

    let language = match matches.value_of("language") {
        None => Language::De,
        Some("en") => Language::En,
        Some("de") => Language::De,
        Some("fr") => Language::Fr,
        Some(lang) => panic!("Unexpected language {}", lang),
    };

    let output_pattern = matches.value_of("output_pattern");

    let input_filenames = matches.values_of("INPUT").unwrap().map(|s| s.to_string()).collect::<Vec<_>>();

    let spec_file = match vsf_filename {
        Some(filename) => {
            let mut f = File::open(filename)?;

            let mut buf = Vec::new();
            let size = f.read_to_end(&mut buf)?;

            SpecificationFile::from_bytes(&buf [0..size])?
        },
        None => {
            SpecificationFile::new_default()
        }
    };

    let spec = Specification::from_file(spec_file, language);

    let (source_type, topology_data_set) = read_topology_data_set(input_filenames.clone(), min_timestamp, max_timestamp)?;

    let flr = FileListReader::new(input_filenames.clone());

    if process_topology_data_set(typ, &topology_data_set, &spec)? {
        // nop, already handled
    } else if source_type == SourceType::Type44 {
        let mut rr = RecordingReader::new(flr);
        rr.set_min_max_timestamps(min_timestamp, max_timestamp);

        let mut config = Config {
            sieve_interval: sieve_interval,
            ttl: ttl_duration,
            min_timestamp: min_timestamp,
            max_timestamp: max_timestamp,
            language: language,
            specification: &spec,
            topology_data_set: &topology_data_set,
            data_set_reader: &mut rr,
            output_pattern: output_pattern,
        };

        if process_data_set_stream(typ, &mut config)? {
            // nop
        } else {
            panic!("Unsupported output type {} for Type 0x44 stream", typ);
        }
    } else if source_type == SourceType::Type88 {
        let mut ldrr = LiveDataRecordingReader::new(flr);
        ldrr.set_min_max_timestamps(min_timestamp, max_timestamp);

        let mut config = Config {
            sieve_interval: sieve_interval,
            ttl: ttl_duration,
            min_timestamp: min_timestamp,
            max_timestamp: max_timestamp,
            language: language,
            specification: &spec,
            topology_data_set: &topology_data_set,
            data_set_reader: &mut ldrr,
            output_pattern: output_pattern,
        };

        if process_data_set_stream(typ, &mut config)? {
            // nop
        } else {
            panic!("Unsupported output type {} for Type 0x88 stream", typ);
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
