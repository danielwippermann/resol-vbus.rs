#![allow(dead_code)]


use resol_vbus::{
    specification::{DataSetPacketField, DataSetPacketFieldIterator},
    *,
};

pub trait FieldIterator<'a> {
    type Iter: Iterator<Item = DataSetPacketField<'a, DataSet>>;

    fn fields_in_data_set(&'a self, data_set: &'a DataSet) -> Self::Iter;
}

pub struct AllFieldsIterator<'a> {
    spec: &'a Specification,
}

impl<'a> AllFieldsIterator<'a> {
    pub fn new(spec: &'a Specification) -> AllFieldsIterator<'a> {
        AllFieldsIterator { spec: spec }
    }
}

impl<'a> FieldIterator<'a> for AllFieldsIterator<'a> {
    type Iter = DataSetPacketFieldIterator<'a, DataSet>;

    fn fields_in_data_set(&'a self, data_set: &'a DataSet) -> Self::Iter {
        self.spec.fields_in_data_set(data_set)
    }
}

pub struct FilteredFieldIteratorImpl<'a> {
    inner: DataSetPacketFieldIterator<'a, DataSet>,
    filters: &'a [PacketFieldId<'a>],
}

impl<'a> Iterator for FilteredFieldIteratorImpl<'a> {
    type Item = DataSetPacketField<'a, DataSet>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let field = self.inner.next()?;

            let is_in_filters = {
                let packet_field_id = field.packet_field_id();
                self.filters.iter().any(|&ref_id| ref_id == packet_field_id)
            };
            if is_in_filters {
                return Some(field);
            }
        }
    }
}

pub struct FilteredFieldIterator<'a> {
    spec: &'a Specification,
    filters: Vec<PacketFieldId<'a>>,
}

impl<'a> FilteredFieldIterator<'a> {
    pub fn new(
        spec: &'a Specification,
        filters: Vec<PacketFieldId<'a>>,
    ) -> FilteredFieldIterator<'a> {
        FilteredFieldIterator {
            spec: spec,
            filters: filters,
        }
    }
}

impl<'a> FieldIterator<'a> for FilteredFieldIterator<'a> {
    type Iter = FilteredFieldIteratorImpl<'a>;

    fn fields_in_data_set(&'a self, data_set: &'a DataSet) -> Self::Iter {
        FilteredFieldIteratorImpl {
            inner: self.spec.fields_in_data_set(data_set),
            filters: &self.filters[..],
        }
    }
}
