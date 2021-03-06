use std::fmt::{Display, Formatter};
use std::io::Write;

use resol_vbus::chrono::Local;

use crate::{
    app_error::Result, config::Config, field_iterator::*,
    timestamp_file_writer::TimestampFileWriter,
};

struct JsonEscape<'a> {
    input: &'a str,
}

impl<'a> JsonEscape<'a> {
    pub fn new(input: &'a str) -> JsonEscape<'a> {
        JsonEscape { input: input }
    }
}

impl<'a> Display for JsonEscape<'a> {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        for c in self.input.chars() {
            match c {
                '\\' | '"' => write!(f, "\\{}", c)?,
                _ => write!(f, "{}", c)?,
            }
        }
        Ok(())
    }
}

pub fn generate(config: &mut Config) -> Result<()> {
    let dsr = &mut config.data_set_reader;
    let spec = config.specification;
    let pattern = config.output_pattern.unwrap_or("Output.json");

    let mut output_writer = TimestampFileWriter::new(pattern.to_owned());

    let field_iterator = AllFieldsIterator::new(spec);

    let output = &mut output_writer;

    if let Some(data_set) = dsr.read_data_set()? {
        let timestamp = data_set.timestamp;
        let local_timestamp = timestamp.with_timezone(&Local);

        output.set_timestamp(timestamp)?;

        write!(output, "{{\n")?;
        write!(
            output,
            "    \"timestamp\": \"{}\",\n",
            local_timestamp.to_rfc3339()
        )?;
        write!(output, "    \"fields\": [")?;
        for (idx, field) in field_iterator
            .fields_in_data_set(&data_set)
            .filter(|field| field.raw_value_i64().is_some())
            .enumerate()
        {
            if idx > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{{\n")?;
            write!(
                output,
                "        \"id\": \"{}_{}\",\n",
                field.packet_spec().packet_id,
                field.field_id()
            )?;
            write!(
                output,
                "        \"packetName\": \"{}\",\n",
                JsonEscape::new(&field.packet_spec().name)
            )?;
            write!(
                output,
                "        \"fieldName\": \"{}\",\n",
                JsonEscape::new(&field.field_spec().name)
            )?;
            write!(
                output,
                "        \"rawValue\": \"{}\",\n",
                field.raw_value_i64().unwrap()
            )?;
            write!(
                output,
                "        \"textValue\": \"{}\",\n",
                field.fmt_raw_value(false)
            )?;
            write!(
                output,
                "        \"unitCode\": \"{}\",\n",
                JsonEscape::new(&field.field_spec().unit_code)
            )?;
            write!(
                output,
                "        \"unitText\": \"{}\"\n",
                JsonEscape::new(&field.field_spec().unit_text)
            )?;
            write!(output, "    }}")?;
        }
        write!(output, "]\n")?;
        write!(output, "}}\n")?;
    } else {
        write!(output, "{{}}\n")?;
    }

    Ok(())
}
