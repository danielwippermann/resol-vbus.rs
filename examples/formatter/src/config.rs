use resol_vbus::{DataSet, Language, Specification};
use resol_vbus::chrono::{DateTime, Duration, UTC};


use data_set_reader::DataSetReader;
// use field_iterator::FieldIterator;


pub struct Config<'a> {
    pub sieve_interval: Option<Duration>,
    pub ttl: Option<Duration>,
    pub min_timestamp: Option<DateTime<UTC>>,
    pub max_timestamp: Option<DateTime<UTC>>,
    pub language: Language,
    pub specification: &'a Specification,
    // pub field_iterator: &'a FieldIterator<'a>,
    pub topology_data_set: &'a DataSet,
    pub data_set_reader: &'a mut DataSetReader,
    pub output_pattern: Option<&'a str>,
}
