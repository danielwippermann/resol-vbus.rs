use resol_vbus::{chrono::Duration, DataSet, Specification};

use crate::data_set_reader::DataSetReader;

pub struct Config<'a> {
    pub sieve_interval: Option<Duration>,
    pub ttl: Option<Duration>,
    // pub min_timestamp: Option<DateTime<Utc>>,
    // pub max_timestamp: Option<DateTime<Utc>>,
    // pub language: Language,
    pub specification: &'a Specification,
    // pub field_iterator: &'a FieldIterator<'a>,
    pub topology_data_set: &'a DataSet,
    pub data_set_reader: &'a mut dyn DataSetReader,
    pub output_pattern: Option<&'a str>,
    pub local_timezone: bool,
}
