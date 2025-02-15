//! Inner VCF header contig map value.

mod builder;
pub mod name;
mod tag;

pub use self::name::Name;

use std::fmt;

use super::{Fields, Indexed, Inner, Map, TryFromFieldsError};

type StandardTag = tag::Standard;
type Tag = super::tag::Tag<StandardTag>;

/// An inner VCF header contig map value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Contig {
    length: Option<usize>,
    idx: Option<usize>,
}

impl Inner for Contig {
    type StandardTag = StandardTag;
    type Builder = builder::Builder;
}

impl Indexed for Contig {
    fn idx(&self) -> Option<usize> {
        self.idx
    }

    fn idx_mut(&mut self) -> &mut Option<usize> {
        &mut self.idx
    }
}

impl Map<Contig> {
    /// Creates a VCF header contig map value.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_vcf::header::record::value::{map::Contig, Map};
    /// let map = Map::<Contig>::new();
    /// # Ok::<_, noodles_vcf::header::record::value::map::contig::name::ParseError>(())
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_vcf::header::record::value::{map::Contig, Map};
    /// let map = Map::<Contig>::new();
    /// assert!(map.length().is_none());
    /// ```
    pub fn length(&self) -> Option<usize> {
        self.inner.length
    }

    /// Returns a mutable reference to the length.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_vcf::header::record::value::{map::Contig, Map};
    ///
    /// let mut map = Map::<Contig>::new();
    /// assert!(map.length().is_none());
    ///
    /// *map.length_mut() = Some(8);
    /// assert_eq!(map.length(), Some(8));
    /// ```
    pub fn length_mut(&mut self) -> &mut Option<usize> {
        &mut self.inner.length
    }
}

impl fmt::Display for Map<Contig> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(length) = self.length() {
            write!(f, ",length={length}")?;
        }

        super::fmt_display_other_fields(f, self.other_fields())?;

        if let Some(idx) = self.idx() {
            super::fmt_display_idx_field(f, idx)?;
        }

        Ok(())
    }
}

impl TryFrom<Fields> for Map<Contig> {
    type Error = TryFromFieldsError;

    fn try_from(fields: Fields) -> Result<Self, Self::Error> {
        let mut other_fields = super::init_other_fields();

        let mut length = None;
        let mut idx = None;

        for (key, value) in fields {
            match Tag::from(key) {
                Tag::Standard(StandardTag::Id) => return Err(TryFromFieldsError::DuplicateTag),
                Tag::Standard(StandardTag::Length) => parse_length(&value, &mut length)?,
                Tag::Standard(StandardTag::Idx) => super::parse_idx(&value, &mut idx)?,
                Tag::Other(t) => super::insert_other_field(&mut other_fields, t, value)?,
            }
        }

        Ok(Self {
            inner: Contig { length, idx },
            other_fields,
        })
    }
}

fn parse_length(s: &str, value: &mut Option<usize>) -> Result<(), TryFromFieldsError> {
    let n = s
        .parse()
        .map_err(|_| TryFromFieldsError::InvalidValue("length"))?;

    if value.replace(n).is_none() {
        Ok(())
    } else {
        Err(TryFromFieldsError::DuplicateTag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() -> Result<(), TryFromFieldsError> {
        let map = Map::<Contig>::try_from(vec![
            (String::from("length"), String::from("8")),
            (
                String::from("md5"),
                String::from("d7eba311421bbc9d3ada44709dd61534"),
            ),
        ])?;

        let expected = r#",length=8,md5="d7eba311421bbc9d3ada44709dd61534""#;
        assert_eq!(map.to_string(), expected);

        Ok(())
    }

    #[test]
    fn test_try_from_fields_for_map_contig() -> Result<(), Box<dyn std::error::Error>> {
        let actual = Map::<Contig>::try_from(Vec::new())?;
        let expected = Map::<Contig>::new();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_parse_length() -> Result<(), TryFromFieldsError> {
        let mut length = None;
        parse_length("8", &mut length)?;
        assert_eq!(length, Some(8));

        assert_eq!(
            parse_length("eight", &mut None),
            Err(TryFromFieldsError::InvalidValue("length"))
        );

        Ok(())
    }
}
