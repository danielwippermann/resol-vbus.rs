//! A converter from the binary recorded VBus file format to human-readable CSV.
extern crate chrono;
extern crate resol_vbus;

use std::fs::File;
use std::io::{Read, Write};

use chrono::{Local};

use resol_vbus::*;


/// Load the VSF file to allow decoding of the payload contained in `Packet` `frame_data` values.
fn load_vsf_file(vsf_filename: &str) -> Specification {
    let mut f = File::open(vsf_filename).unwrap();

    let mut buf = Vec::new();
    let size = f.read_to_end(&mut buf).unwrap();

    let spec_file = SpecificationFile::from_bytes(&buf [0..size]).unwrap();

    let spec = Specification::from_file(spec_file, Language::En);

    spec
}


/// Read the "*.vbus" files a second time to convert / process the `DataSet` values within.
fn print_data(file_list: Vec<String>, topology_data_set: DataSet, spec: &Specification) {
    let flr = FileListReader::new(file_list);

    let mut rr = LiveDataRecordingReader::new(flr);

    let mut cumultative_data_set = topology_data_set;

    let mut output = std::io::stdout();

    while let Some(data) = rr.read_data().unwrap() {
        let timestamp = data.as_ref().timestamp;
        let local_timestamp = timestamp.with_timezone(&Local);

        cumultative_data_set.add_data(data);

        write!(output, "{}", local_timestamp).unwrap();

        for field in spec.fields_in_data_set(&cumultative_data_set.as_ref()) {
            write!(output, "\t{}", field.fmt_raw_value(false)).unwrap();
        }

        write!(output, "\n").unwrap();
    }
}


fn main() {
    let vsf_filename = "vbus_specification.vsf";

    let file_list = std::env::args().skip(1).map(|arg| arg.to_owned()).collect::<Vec<_>>();

    // Load the VSF file to allow decoding of `Packet` values.
    let spec = load_vsf_file(vsf_filename);

    // Read the "*.vbus" files once to find all unique `Packet` values. This allows to
    // only generate CSV column headers once and keep the column layout stable throughout
    // the conversion.
    let flr = FileListReader::new(file_list.clone());

    let mut rr = LiveDataRecordingReader::new(flr);

    let topology_data_set = rr.read_topology_data_set().unwrap();
    for data in topology_data_set.as_data_slice() {
        println!("{}: {:?}", data.id_string(), data);
    }

    // Read the "*.vbus" files a second time to convert / process the `DataSet` values within.
    print_data(file_list, topology_data_set, &spec);
}
