use std::fmt;

use indexmap::IndexMap;

use super::{builder, tag, Fields, Inner, Map, TryFromFieldsError};

type StandardTag = tag::Identity;
type Tag = tag::Tag<StandardTag>;

/// An inner VCF header other map value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Other {
    id: String,
}

impl Inner for Other {
    type StandardTag = StandardTag;
    type Builder = builder::Identity;
}

impl Map<Other> {
    /// Creates a nonstandard VCF header map value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_vcf::header::record::value::{map::Other, Map};
    /// let map = Map::<Other>::new("noodles");
    /// ```
    pub fn new<I>(id: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            inner: Other { id: id.into() },
            other_fields: IndexMap::new(),
        }
    }
}

impl fmt::Display for Map<Other> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::fmt_display_other_fields(f, self.other_fields())
    }
}

impl TryFrom<Fields> for Map<Other> {
    type Error = TryFromFieldsError;

    fn try_from(fields: Fields) -> Result<Self, Self::Error> {
        let mut other_fields = super::init_other_fields();

        let mut id = None;

        for (key, value) in fields {
            match Tag::from(key) {
                Tag::Standard(StandardTag::Id) => super::parse_id(&value, &mut id)?,
                Tag::Other(t) => super::insert_other_field(&mut other_fields, t, value)?,
            }
        }

        let id: String = id.ok_or(TryFromFieldsError::MissingField("ID"))?;

        Ok(Self {
            inner: Other { id },
            other_fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() {
        let map = Map::<Other>::new("noodles");
        assert!(map.to_string().is_empty());
    }

    #[test]
    fn test_try_from_fields_for_map_other() -> Result<(), TryFromFieldsError> {
        let actual = Map::<Other>::try_from(vec![(String::from("ID"), String::from("noodles"))])?;
        let expected = Map::<Other>::new("noodles");
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_try_from_fields_for_info_with_missing_fields() {
        assert_eq!(
            Map::<Other>::try_from(Vec::new()),
            Err(TryFromFieldsError::MissingField("ID"))
        );
    }
}
