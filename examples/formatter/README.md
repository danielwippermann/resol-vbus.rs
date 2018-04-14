# resol-vbus.rs formatter example

This example takes files in the [VBus Recording File Format]() as input and
passes their contents to one of the supported formatting processors:

- `stats` print out stats about the data found within the input files
- `packets` prints out a list of `PacketId`s found within the input files
- `fields` prints out a list of `PacketFieldId`s found within the input files
- `filter-template` prints out a Rust code sample implementing a
  `FilteredFieldIterator` based on all the `PacketFieldId`s found within the
  input files
- `csv` converts the input files to one or more CSV files
- `simple-json` converts the first data set from the input file to a simple
  JSON file


## Compile

The formatter example is part of the `resol-vbus.rs` repo:

	git clone https://github.com/danielwippermann/resol-vbus.rs
	cd resol-vbus.rs/examples/formatter
	cargo build


## Run

You can either use `cargo run`:

	cargo run -- <args...>

or run the built executable directly:

	target/debug/formatter <args...>


## Examples

The following examples process 16 months of data (approx. 160 MB).


### Print stats

	$ time target/release/formatter stats test-data/*_packets.vbus
	Min. timestamp: 2016-08-06T00:00:00.315+00:00
	Max. timestamp: 2017-12-14T17:40:00.435+00:00
	Data set count: 140625
	Data count: 2270507
	Data IDs:
	- 00_0010_0053_10_0100: DL3
	- 01_0010_7E11_10_0100: VBus 1: DeltaSol MX [Regler]
	- 01_0010_7E12_10_0100: VBus 1: DeltaSol MX [Module]
	- 01_0010_7E21_10_0100: VBus 1: DeltaSol MX [Heizkreis #1]
	- 01_0010_7E22_10_0100: VBus 1: DeltaSol MX [Heizkreis #2]
	- 01_0010_7E31_10_0100: VBus 1: DeltaSol MX [WMZ #1]
	- 01_0010_7E32_10_0100: VBus 1: DeltaSol MX [WMZ #2]
	- 01_0010_7E33_10_0100: VBus 1: DeltaSol MX [WMZ #3]
	- 01_0010_7E34_10_0100: VBus 1: DeltaSol MX [WMZ #4]
	- 01_0010_7E35_10_0100: VBus 1: DeltaSol MX [WMZ #5]
	- 01_0015_7E11_10_0100: VBus 1: DeltaSol MX [Regler] => VBus 1: Standard-Infos
	- 01_6651_7E11_10_0200: VBus 1: DeltaSol MX [Regler] => VBus 1: EM #1
	- 01_6652_7E11_10_0200: VBus 1: DeltaSol MX [Regler] => VBus 1: EM #2
	- 01_6653_7E11_10_0200: VBus 1: DeltaSol MX [Regler] => VBus 1: EM #3
	- 01_6654_7E11_10_0200: VBus 1: DeltaSol MX [Regler] => VBus 1: EM #4
	- 01_6655_7E11_10_0200: VBus 1: DeltaSol MX [Regler] => VBus 1: EM #5
	- 01_7E11_6651_10_0100: VBus 1: EM #1 => VBus 1: DeltaSol MX [Regler]
	target/release/formatter stats   0,50s user 0,14s system 70% cpu 0,904 total


### Print list of `PacketIds`

	$ time target/release/formatter packets test-data/*_packets.vbus
	PacketId(0x00, 0x0010, 0x0053, 0x0100),  // DL3
	PacketId(0x01, 0x0010, 0x7E11, 0x0100),  // VBus 1: DeltaSol MX [Regler]
	PacketId(0x01, 0x0010, 0x7E12, 0x0100),  // VBus 1: DeltaSol MX [Module]
	PacketId(0x01, 0x0010, 0x7E21, 0x0100),  // VBus 1: DeltaSol MX [Heizkreis #1]
	PacketId(0x01, 0x0010, 0x7E22, 0x0100),  // VBus 1: DeltaSol MX [Heizkreis #2]
	PacketId(0x01, 0x0010, 0x7E31, 0x0100),  // VBus 1: DeltaSol MX [WMZ #1]
	PacketId(0x01, 0x0010, 0x7E32, 0x0100),  // VBus 1: DeltaSol MX [WMZ #2]
	PacketId(0x01, 0x0010, 0x7E33, 0x0100),  // VBus 1: DeltaSol MX [WMZ #3]
	PacketId(0x01, 0x0010, 0x7E34, 0x0100),  // VBus 1: DeltaSol MX [WMZ #4]
	PacketId(0x01, 0x0010, 0x7E35, 0x0100),  // VBus 1: DeltaSol MX [WMZ #5]
	PacketId(0x01, 0x6651, 0x7E11, 0x0200),  // VBus 1: DeltaSol MX [Regler] => VBus 1: EM #1
	PacketId(0x01, 0x6652, 0x7E11, 0x0200),  // VBus 1: DeltaSol MX [Regler] => VBus 1: EM #2
	PacketId(0x01, 0x6653, 0x7E11, 0x0200),  // VBus 1: DeltaSol MX [Regler] => VBus 1: EM #3
	PacketId(0x01, 0x6654, 0x7E11, 0x0200),  // VBus 1: DeltaSol MX [Regler] => VBus 1: EM #4
	PacketId(0x01, 0x6655, 0x7E11, 0x0200),  // VBus 1: DeltaSol MX [Regler] => VBus 1: EM #5
	PacketId(0x01, 0x7E11, 0x6651, 0x0100),  // VBus 1: EM #1 => VBus 1: DeltaSol MX [Regler]
	target/release/formatter packets   0,15s user 0,06s system 65% cpu 0,332 total


### Generate CSV files

	time target/release/formatter --output Output-%Y-%m.csv csv test-data/*_packets.vbus
	Generating "Output-2016-08.csv"...
	Generating "Output-2016-09.csv"...
	Generating "Output-2016-10.csv"...
	Generating "Output-2016-11.csv"...
	Generating "Output-2016-12.csv"...
	Generating "Output-2017-01.csv"...
	Generating "Output-2017-02.csv"...
	Generating "Output-2017-03.csv"...
	Generating "Output-2017-04.csv"...
	Generating "Output-2017-05.csv"...
	Generating "Output-2017-06.csv"...
	Generating "Output-2017-07.csv"...
	Generating "Output-2017-08.csv"...
	Generating "Output-2017-09.csv"...
	Generating "Output-2017-10.csv"...
	Generating "Output-2017-11.csv"...
	Generating "Output-2017-12.csv"...
	target/release/formatter --output Output-%Y-%m.csv csv   89,25s user 563,09s system 96% cpu 11:15,82 total
