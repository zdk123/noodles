use std::{
    cmp,
    ffi::CString,
    io::{self, Write},
    mem,
};

use byteorder::{LittleEndian, WriteBytesExt};
use noodles_bgzf as bgzf;
use noodles_sam::{
    self as sam,
    header::{ReferenceSequence, ReferenceSequences},
    record::{Cigar, Data, MateReferenceSequenceName, QualityScores, Sequence},
};

use crate::record::sequence::Base;

use super::MAGIC_NUMBER;

// § 4.2 The BAM format (2020-04-30)
//
// ref_id (4) + pos (4) + l_read_name (1) + mapq (1) + bin (2) + n_cigar_op (2) + flag (2) + l_seq
// (4) + next_ref_id (4) + next_pos (4) + tlen (4)
const BLOCK_HEADER_SIZE: usize = 32;

// § 4.2.1 BIN field calculation (2020-04-30)
const UNMAPPED_BIN: u16 = 4680;

// § 4.2.3 SEQ and QUAL encoding (2020-04-30)
const NULL_QUALITY_SCORE: u8 = 255;

/// A BAM writer.
///
/// Since the raw text header and `bam::Record` are immutable, BAM files are created by encoding a
/// SAM header and SAM records.
///
/// # Examples
///
/// ```no_run
/// # use std::io;
/// use noodles_bam as bam;
/// use noodles_sam as sam;
///
/// let mut writer = bam::Writer::new(Vec::new());
///
/// let header = sam::Header::builder().add_comment("noodles-bam").build();
/// writer.write_header(&header)?;
/// writer.write_reference_sequences(header.reference_sequences())?;
///
/// let record = sam::Record::default();
/// writer.write_record(header.reference_sequences(), &record)?;
/// # Ok::<(), io::Error>(())
/// ```
pub struct Writer<W>
where
    W: Write,
{
    inner: bgzf::Writer<W>,
}

impl<W> Writer<W>
where
    W: Write,
{
    /// Creates a new writer with a default compression level.
    ///
    /// The given stream is wrapped in a BGZF encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_bam as bam;
    /// let writer = bam::Writer::new(Vec::new());
    /// ```
    pub fn new(writer: W) -> Self {
        Self {
            inner: bgzf::Writer::new(writer),
        }
    }

    /// Returns a reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_bam as bam;
    /// let writer = bam::Writer::new(Vec::new());
    /// assert!(writer.get_ref().is_empty());
    /// ```
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    /// Attempts to finish the output stream.
    ///
    /// This is typically only manually called if the underlying stream is needed before the writer
    /// is dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam as bam;
    /// let mut writer = bam::Writer::new(Vec::new());
    /// writer.try_finish()?;
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn try_finish(&mut self) -> io::Result<()> {
        self.inner.try_finish()
    }

    /// Writes a SAM header.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam as bam;
    /// use noodles_sam as sam;
    ///
    /// let mut writer = bam::Writer::new(Vec::new());
    ///
    /// let header = sam::Header::builder().add_comment("noodles-bam").build();
    /// writer.write_header(&header)?;
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn write_header(&mut self, header: &sam::Header) -> io::Result<()> {
        self.inner.write_all(MAGIC_NUMBER)?;

        let text = header.to_string();
        let l_text = text.len() as i32;
        self.inner.write_i32::<LittleEndian>(l_text)?;

        self.inner.write_all(text.as_bytes())?;

        Ok(())
    }

    /// Writes SAM reference sequences.
    ///
    /// The reference sequences here are typically the same as the reference sequences in the SAM
    /// header.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam as bam;
    /// use noodles_sam as sam;
    ///
    /// let mut writer = bam::Writer::new(Vec::new());
    ///
    /// let header = sam::Header::builder()
    ///     .add_reference_sequence(sam::header::ReferenceSequence::new(String::from("sq0"), 8))
    ///     .add_comment("noodles-bam")
    ///     .build();
    ///
    /// writer.write_header(&header)?;
    /// writer.write_reference_sequences(header.reference_sequences())?;
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn write_reference_sequences(
        &mut self,
        reference_sequences: &ReferenceSequences,
    ) -> io::Result<()> {
        let n_ref = reference_sequences.len() as i32;
        self.inner.write_i32::<LittleEndian>(n_ref)?;

        for reference_sequence in reference_sequences.values() {
            write_reference(&mut self.inner, reference_sequence)?;
        }

        Ok(())
    }

    /// Writes a SAM record.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam as bam;
    /// use noodles_sam as sam;
    ///
    /// let mut writer = bam::Writer::new(Vec::new());
    ///
    /// let reference_sequences = sam::header::ReferenceSequences::new();
    /// let record = sam::Record::default();
    /// writer.write_record(&reference_sequences, &record)?;
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn write_record(
        &mut self,
        reference_sequences: &ReferenceSequences,
        record: &sam::Record,
    ) -> io::Result<()> {
        let c_read_name = CString::new(record.read_name().as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let reference_sequence_id = match &**record.reference_sequence_name() {
            Some(name) => reference_sequences
                .get_full(name)
                .map(|(i, _, _)| i as i32)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid reference sequence id")
                })?,
            None => -1,
        };

        let mate_reference_sequence_id = match record.mate_reference_sequence_name() {
            MateReferenceSequenceName::Some(name) => reference_sequences
                .get_full(name)
                .map(|(i, _, _)| i as i32)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid reference sequence id")
                })?,
            MateReferenceSequenceName::Eq => reference_sequence_id,
            MateReferenceSequenceName::None => -1,
        };

        let read_name = c_read_name.as_bytes_with_nul();
        let l_read_name = read_name.len() as u8;
        let n_cigar_op = record.cigar().len() as u16;
        let l_seq = record.sequence().len() as i32;
        let data_len = calculate_data_len(record.data()) as i32;

        let block_size = BLOCK_HEADER_SIZE as i32
            + (l_read_name as i32)
            + (4 * (n_cigar_op as i32))
            + ((l_seq + 1) / 2)
            + l_seq
            + data_len;

        self.inner.write_i32::<LittleEndian>(block_size)?;

        let ref_id = reference_sequence_id as i32;
        self.inner.write_i32::<LittleEndian>(ref_id)?;

        let pos = i32::from(record.position()) - 1;
        self.inner.write_i32::<LittleEndian>(pos)?;

        self.inner.write_u8(l_read_name)?;

        let mapq = u8::from(record.mapping_quality());
        self.inner.write_u8(mapq)?;

        let bin = record
            .position()
            .map(|start| {
                // 0-based, [start, end)
                let reference_len = record.cigar().reference_len() as i32;
                let end = start + reference_len;
                region_to_bin(start, end) as u16
            })
            .unwrap_or(UNMAPPED_BIN);

        self.inner.write_u16::<LittleEndian>(bin)?;

        self.inner.write_u16::<LittleEndian>(n_cigar_op)?;

        let flag = u16::from(record.flags());
        self.inner.write_u16::<LittleEndian>(flag)?;

        self.inner.write_i32::<LittleEndian>(l_seq)?;

        let next_ref_id = mate_reference_sequence_id as i32;
        self.inner.write_i32::<LittleEndian>(next_ref_id)?;

        let next_pos = i32::from(record.mate_position()) - 1;
        self.inner.write_i32::<LittleEndian>(next_pos)?;

        let tlen = record.template_len();
        self.inner.write_i32::<LittleEndian>(tlen)?;

        self.inner.write_all(read_name)?;

        write_cigar(&mut self.inner, record.cigar())?;

        // § 4.2.3 SEQ and QUAL encoding (2020-04-30)
        let sequence = record.sequence();
        let quality_scores = record.quality_scores();

        write_seq(&mut self.inner, sequence)?;

        match sequence.len().cmp(&quality_scores.len()) {
            cmp::Ordering::Less => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "quality scores length does not match sequence length",
                ));
            }
            cmp::Ordering::Greater => {
                if quality_scores.is_empty() {
                    for _ in 0..sequence.len() {
                        self.inner.write_u8(NULL_QUALITY_SCORE)?;
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "quality scores length does not match sequence length",
                    ));
                }
            }
            cmp::Ordering::Equal => {
                write_qual(&mut self.inner, quality_scores)?;
            }
        }

        write_data(&mut self.inner, record.data())?;

        Ok(())
    }
}

impl<W> Drop for Writer<W>
where
    W: Write,
{
    fn drop(&mut self) {
        let _ = self.try_finish();
    }
}

fn write_reference<W>(writer: &mut W, reference_sequence: &ReferenceSequence) -> io::Result<()>
where
    W: Write,
{
    let c_name = CString::new(reference_sequence.name())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let name = c_name.as_bytes_with_nul();

    let l_name = name.len() as i32;
    writer.write_i32::<LittleEndian>(l_name)?;
    writer.write_all(name)?;

    let l_ref = reference_sequence.len() as i32;
    writer.write_i32::<LittleEndian>(l_ref)?;

    Ok(())
}

fn write_cigar<W>(writer: &mut W, cigar: &Cigar) -> io::Result<()>
where
    W: Write,
{
    for op in cigar.iter() {
        let len = op.len() as u32;
        let kind = op.kind() as u32;
        let value = len << 4 | kind;
        writer.write_u32::<LittleEndian>(value)?;
    }

    Ok(())
}

fn write_seq<W>(writer: &mut W, sequence: &Sequence) -> io::Result<()>
where
    W: Write,
{
    for chunk in sequence.chunks(2) {
        let l = Base::from(chunk[0]);

        let r = if let Some(c) = chunk.get(1) {
            Base::from(*c)
        } else {
            Base::Eq
        };

        let value = (l as u8) << 4 | (r as u8);

        writer.write_u8(value)?;
    }

    Ok(())
}

fn write_qual<W>(writer: &mut W, quality_scores: &QualityScores) -> io::Result<()>
where
    W: Write,
{
    for score in quality_scores.scores() {
        let value = u8::from(*score);
        writer.write_u8(value)?;
    }

    Ok(())
}

fn calculate_data_len(data: &Data) -> usize {
    use noodles_sam::record::data::field::Value;

    let mut len = 0;

    for field in data.iter() {
        // tag
        len += 2;
        // val_type
        len += 1;

        let value = field.value();

        if value.subtype().is_some() {
            // subtype
            len += 1;
            // count
            len += mem::size_of::<u32>();
        }

        match value {
            Value::Char(_) => {
                len += mem::size_of::<u8>();
            }
            Value::Int32(_) => {
                len += mem::size_of::<i32>();
            }
            Value::Float(_) => {
                len += mem::size_of::<f32>();
            }
            Value::String(s) | Value::Hex(s) => {
                len += s.as_bytes().len() + 1;
            }
            Value::Int8Array(values) => {
                len += values.len();
            }
            Value::UInt8Array(values) => {
                len += values.len();
            }
            Value::Int16Array(values) => {
                len += mem::size_of::<i16>() * values.len();
            }
            Value::UInt16Array(values) => {
                len += mem::size_of::<u16>() * values.len();
            }
            Value::Int32Array(values) => {
                len += mem::size_of::<i32>() * values.len();
            }
            Value::UInt32Array(values) => {
                len += mem::size_of::<u32>() * values.len();
            }
            Value::FloatArray(values) => {
                len += mem::size_of::<f32>() * values.len();
            }
        }
    }

    len
}

fn write_data<W>(writer: &mut W, data: &Data) -> io::Result<()>
where
    W: Write,
{
    use noodles_sam::record::data::field::Value;

    for field in data.iter() {
        writer.write_all(field.tag().as_ref().as_bytes())?;

        let value = field.value();
        writer.write_u8(char::from(value.ty()) as u8)?;

        if let Some(subtype) = value.subtype() {
            writer.write_u8(char::from(subtype) as u8)?;
        }

        match value {
            Value::Char(c) => {
                writer.write_u8(*c as u8)?;
            }
            Value::Int32(n) => {
                writer.write_i32::<LittleEndian>(*n)?;
            }
            Value::Float(n) => {
                writer.write_f32::<LittleEndian>(*n)?;
            }
            Value::String(s) | Value::Hex(s) => {
                let c_str = CString::new(s.as_bytes())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                writer.write_all(c_str.as_bytes_with_nul())?;
            }
            Value::Int8Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_i8(n)?;
                }
            }
            Value::UInt8Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_u8(n)?;
                }
            }
            Value::Int16Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_i16::<LittleEndian>(n)?;
                }
            }
            Value::UInt16Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_u16::<LittleEndian>(n)?;
                }
            }
            Value::Int32Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_i32::<LittleEndian>(n)?;
                }
            }
            Value::UInt32Array(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_u32::<LittleEndian>(n)?;
                }
            }
            Value::FloatArray(values) => {
                writer.write_u32::<LittleEndian>(values.len() as u32)?;

                for &n in values {
                    writer.write_f32::<LittleEndian>(n)?;
                }
            }
        }
    }

    Ok(())
}

// § 5.3 C source code for computing bin number and overlapping bins (2020-04-30)
// 0-based, [start, end)
#[allow(clippy::eq_op)]
fn region_to_bin(start: i32, mut end: i32) -> i32 {
    end -= 1;

    if start >> 14 == end >> 14 {
        ((1 << 15) - 1) / 7 + (start >> 14)
    } else if start >> 17 == end >> 17 {
        ((1 << 12) - 1) / 7 + (start >> 17)
    } else if start >> 20 == end >> 20 {
        ((1 << 9) - 1) / 7 + (start >> 20)
    } else if start >> 23 == end >> 23 {
        ((1 << 6) - 1) / 7 + (start >> 23)
    } else if start >> 26 == end >> 26 {
        ((1 << 3) - 1) / 7 + (start >> 26)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use crate::{Reader, Record};

    use super::*;

    #[test]
    fn test_write_header() -> io::Result<()> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::builder()
            .set_header(sam::header::header::Header::default())
            .build();

        writer.write_header(&header)?;
        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());
        let actual = reader.read_header()?;

        let expected = "@HD\tVN:1.6\n";

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_write_reference_sequences() -> io::Result<()> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::builder()
            .add_reference_sequence(sam::header::ReferenceSequence::new(String::from("sq0"), 8))
            .set_header(sam::header::header::Header::default())
            .build();

        writer.write_header(&header)?;
        writer.write_reference_sequences(header.reference_sequences())?;
        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());
        reader.read_header()?;
        let actual = reader.read_reference_sequences()?;

        assert_eq!(actual.len(), 1);
        assert_eq!(
            &actual[0],
            &sam::header::ReferenceSequence::new(String::from("sq0"), 8)
        );

        Ok(())
    }

    #[test]
    fn test_write_record() -> io::Result<()> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let sam_record = sam::Record::default();
        writer.write_record(header.reference_sequences(), &sam_record)?;
        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());

        let mut record = Record::default();
        reader.read_record(&mut record)?;

        assert_eq!(record.read_name(), b"*\0");
        assert_eq!(record.flags(), sam::record::Flags::default());
        assert!(record.reference_sequence_id().is_none());
        assert!(record.position().is_none());
        assert_eq!(
            record.mapping_quality(),
            sam::record::MappingQuality::from(255)
        );
        assert!(record.cigar().is_empty());
        assert!(record.mate_reference_sequence_id().is_none());
        assert!(record.mate_position().is_none());
        assert_eq!(record.template_len(), 0);
        assert!(record.sequence().is_empty());
        assert!(record.quality_scores().is_empty());
        assert!(record.data().is_empty());

        Ok(())
    }

    #[test]
    fn test_write_record_with_sequence_length_less_than_quality_scores_length(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let record = sam::Record::builder()
            .set_sequence("AT".parse()?)
            .set_quality_scores("NDLS".parse()?)
            .build();

        assert!(writer
            .write_record(header.reference_sequences(), &record)
            .is_err());

        Ok(())
    }

    #[test]
    fn test_write_record_with_sequence_length_greater_than_quality_scores_length(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let record = sam::Record::builder()
            .set_sequence("ATCG".parse()?)
            .set_quality_scores("ND".parse()?)
            .build();

        assert!(writer
            .write_record(header.reference_sequences(), &record)
            .is_err());

        Ok(())
    }

    #[test]
    fn test_write_record_with_sequence_and_no_quality_scores(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let sam_record = sam::Record::builder().set_sequence("ATCG".parse()?).build();
        writer.write_record(header.reference_sequences(), &sam_record)?;

        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());

        let mut record = Record::default();
        reader.read_record(&mut record)?;

        let actual: Vec<_> = record.sequence().bases().collect();
        let expected = [Base::A, Base::T, Base::C, Base::G];
        assert_eq!(actual, expected);

        let actual = record.quality_scores();
        let expected = [255, 255, 255, 255];
        assert_eq!(*actual, expected);

        Ok(())
    }

    #[test]
    fn test_write_record_with_sequence_and_quality_scores() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let sam_record = sam::Record::builder()
            .set_sequence("ATCG".parse()?)
            .set_quality_scores("NDLS".parse()?)
            .build();

        writer.write_record(header.reference_sequences(), &sam_record)?;
        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());

        let mut record = Record::default();
        reader.read_record(&mut record)?;

        let actual: Vec<_> = record.sequence().bases().collect();
        let expected = [Base::A, Base::T, Base::C, Base::G];
        assert_eq!(actual, expected);

        let actual = record.quality_scores();
        let expected = [45, 35, 43, 50];
        assert_eq!(*actual, expected);

        Ok(())
    }

    #[test]
    fn test_write_record_with_data() -> io::Result<()> {
        use noodles_sam::record::data::{
            field::{Tag as SamTag, Value as SamValue},
            Field as SamField,
        };

        use crate::record::data::{field::Value, Field};

        let mut writer = Writer::new(Vec::new());

        let header = sam::Header::default();
        let sam_record = sam::Record::builder()
            .set_data(Data::from(vec![
                SamField::new(SamTag::ReadGroup, SamValue::String(String::from("rg0"))),
                SamField::new(SamTag::AlignmentHitCount, SamValue::Int32(1)),
            ]))
            .build();

        writer.write_record(header.reference_sequences(), &sam_record)?;
        writer.try_finish()?;

        let mut reader = Reader::new(writer.get_ref().as_slice());

        let mut record = Record::default();
        reader.read_record(&mut record)?;

        let bam_data = record.data();
        let mut fields = bam_data.fields();

        assert_eq!(
            fields.next().transpose()?,
            Some(Field::new(
                SamTag::ReadGroup,
                Value::String(String::from("rg0"))
            ),)
        );

        assert_eq!(
            fields.next().transpose()?,
            Some(Field::new(SamTag::AlignmentHitCount, Value::Int32(1)))
        );

        assert!(fields.next().is_none());

        Ok(())
    }

    #[test]
    fn test_region_to_bin() {
        // [8, 13]
        assert_eq!(region_to_bin(7, 13), 4681);
        // [63245986, 63245986]
        assert_eq!(region_to_bin(63245985, 63255986), 8541);
    }
}
