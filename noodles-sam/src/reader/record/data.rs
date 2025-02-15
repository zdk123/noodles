pub(crate) mod field;

use std::io;

use self::field::parse_field;
use crate::record::Data;

pub(crate) fn parse_data(mut src: &[u8]) -> io::Result<Data> {
    let mut data = Data::default();

    while let Some((tag, value)) = parse_field(&mut src)? {
        data.insert(tag, value);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data() -> Result<(), Box<dyn std::error::Error>> {
        use crate::record::data::field::{Tag, Value};

        assert!(parse_data(b"")?.is_empty());

        let nh = (Tag::AlignmentHitCount, Value::from(1u8));
        let co = (Tag::Comment, Value::String(String::from("ndls")));

        let actual = parse_data(b"NH:i:1")?;
        let expected = [nh.clone()].into_iter().collect();
        assert_eq!(actual, expected);

        let actual = parse_data(b"NH:i:1\tCO:Z:ndls")?;
        let expected = [nh, co].into_iter().collect();
        assert_eq!(actual, expected);

        Ok(())
    }
}
