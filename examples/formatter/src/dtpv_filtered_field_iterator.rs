use resol_vbus::{PacketFieldId, PacketId, Specification};

use crate::field_iterator::FilteredFieldIterator;

pub fn create_dtpv_filtered_field_iterator<'a>(
    spec: &'a Specification,
) -> FilteredFieldIterator<'a> {
    let channel = 2;

    FilteredFieldIterator::new(
        spec,
        vec![
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_1"),  // Funktionsstatus Aus
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_2"),  // Funktionsstatus Fehler
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_4"),  // Funktionsstatus Bereit
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_8"),  // Funktionsstatus Heizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_16"),  // Funktionsstatus Max. Temp.
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_32"),  // Funktionsstatus Lstg. reduziert
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "000_1_64"),  // Funktionsstatus Nachheizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "002_4_0"),  // Leistung Überschuss
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "006_4_0"),  // Leistung Heizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "068_2_0"),  // DCIn
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "010_2_0"),  // Temperatur Speicher (Sensor 1)
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "012_2_0"),  // Temperatur Sensor 2
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "014_2_0"),  // Temperatur Sensor 3
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "016_4_0"),  // Überschuss
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "024_4_0"),  // Heizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "032_4_0"),  // Betriebsstunden Heizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "056_4_0"),  // Nachheizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "064_4_0"),  // Betriebsstunden Nachheizung
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "036_2_0"),  // Parameter Max. Temp. (S1)
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "040_2_0"),  // Parameter Reserve
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "044_4_0"),  // Systemdatum
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "048_1_2"),  // !Sensormodul Bus-Kommunikation gestört
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "048_1_1"),  // !Sensorfehler
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "048_1_4"),  // !Lüfterfehler
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "048_1_8"),  // !Max. Temp. Regler
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "048_1_16"),  // !Datum/Uhrzeit
            PacketFieldId(PacketId(0x02, 0x0010, 0x111E, 0x0100), "052_1_4"),  // !Lüfterwarnung
        ],
    )
}
