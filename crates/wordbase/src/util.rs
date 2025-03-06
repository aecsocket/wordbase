use std::fmt;

macro_rules! display_as_serialize {
    ($T:ty) => {
        const _: () = {
            use std::fmt;

            impl fmt::Display for $T {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    let serializer = crate::util::FormatterSerializer { f };
                    self.serialize(serializer)
                }
            }
        };
    };
}

pub(crate) use display_as_serialize;

pub struct FormatterSerializer<'a, 'b> {
    pub f: &'a mut fmt::Formatter<'b>,
}

impl serde::Serializer for FormatterSerializer<'_, '_> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        write!(self.f, "{variant}")
    }

    serde::__serialize_unimplemented! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str bytes none some
        unit unit_struct newtype_struct newtype_variant
        seq tuple tuple_struct tuple_variant map struct struct_variant
    }
}
