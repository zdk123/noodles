use noodles_bam as bam;
use noodles_sam as sam;

use super::{Feature, Flags, NextMateFlags, ReadGroupId, Record, Tag};

pub struct Builder {
    id: i64,
    bam_flags: sam::record::Flags,
    flags: Flags,
    reference_sequence_id: Option<bam::record::ReferenceSequenceId>,
    read_length: i32,
    alignment_start: i32,
    read_group_id: ReadGroupId,
    read_name: Vec<u8>,
    next_mate_flags: NextMateFlags,
    next_fragment_reference_sequence_id: Option<bam::record::ReferenceSequenceId>,
    next_mate_alignment_start: i32,
    template_size: i32,
    distance_to_next_fragment: i32,
    tags: Vec<Tag>,
    bases: Vec<u8>,
    features: Vec<Feature>,
    mapping_quality: sam::record::MappingQuality,
    quality_scores: Vec<u8>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            id: 0,
            bam_flags: sam::record::Flags::UNMAPPED,
            flags: Flags::default(),
            reference_sequence_id: None,
            read_length: 0,
            alignment_start: 0,
            read_group_id: ReadGroupId::default(),
            read_name: Vec::new(),
            next_mate_flags: NextMateFlags::default(),
            next_fragment_reference_sequence_id: None,
            next_mate_alignment_start: 0,
            template_size: 0,
            distance_to_next_fragment: 0,
            tags: Vec::new(),
            bases: Vec::new(),
            features: Vec::new(),
            mapping_quality: sam::record::MappingQuality::default(),
            quality_scores: Vec::new(),
        }
    }
}

impl Builder {
    pub fn set_id(mut self, id: i64) -> Self {
        self.id = id;
        self
    }

    pub fn set_bam_flags(mut self, bam_flags: sam::record::Flags) -> Self {
        self.bam_flags = bam_flags;
        self
    }

    pub fn set_flags(mut self, flags: Flags) -> Self {
        self.flags = flags;
        self
    }

    pub fn set_reference_sequence_id(
        mut self,
        reference_sequence_id: bam::record::ReferenceSequenceId,
    ) -> Self {
        self.reference_sequence_id = Some(reference_sequence_id);
        self
    }

    pub fn set_read_length(mut self, read_length: i32) -> Self {
        self.read_length = read_length;
        self
    }

    pub fn set_alignment_start(mut self, alignment_start: i32) -> Self {
        self.alignment_start = alignment_start;
        self
    }

    pub fn set_read_group_id(mut self, read_group_id: ReadGroupId) -> Self {
        self.read_group_id = read_group_id;
        self
    }

    pub fn set_read_name(mut self, read_name: Vec<u8>) -> Self {
        self.read_name = read_name;
        self
    }

    pub fn set_next_mate_flags(mut self, next_mate_flags: NextMateFlags) -> Self {
        self.next_mate_flags = next_mate_flags;
        self
    }

    pub fn set_next_fragment_reference_sequence_id(
        mut self,
        next_fragment_reference_sequence_id: bam::record::ReferenceSequenceId,
    ) -> Self {
        self.next_fragment_reference_sequence_id = Some(next_fragment_reference_sequence_id);
        self
    }

    pub fn set_next_mate_alignment_start(mut self, next_mate_alignment_start: i32) -> Self {
        self.next_mate_alignment_start = next_mate_alignment_start;
        self
    }

    pub fn set_template_size(mut self, template_size: i32) -> Self {
        self.template_size = template_size;
        self
    }

    pub fn set_distance_to_next_fragment(mut self, distance_to_next_fragment: i32) -> Self {
        self.distance_to_next_fragment = distance_to_next_fragment;
        self
    }

    pub fn set_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn set_bases(mut self, bases: Vec<u8>) -> Self {
        self.bases = bases;
        self
    }

    pub fn add_base(mut self, base: u8) -> Self {
        self.bases.push(base);
        self
    }

    pub fn set_features(mut self, features: Vec<Feature>) -> Self {
        self.features = features;
        self
    }

    pub fn add_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }

    pub fn set_mapping_quality(mut self, mapping_quality: sam::record::MappingQuality) -> Self {
        self.mapping_quality = mapping_quality;
        self
    }

    pub fn set_quality_scores(mut self, quality_scores: Vec<u8>) -> Self {
        self.quality_scores = quality_scores;
        self
    }

    pub fn add_quality_score(mut self, quality_score: u8) -> Self {
        self.quality_scores.push(quality_score);
        self
    }

    pub fn build(self) -> Record {
        Record {
            id: self.id,
            bam_bit_flags: self.bam_flags,
            cram_bit_flags: self.flags,
            reference_sequence_id: self.reference_sequence_id,
            read_length: self.read_length,
            alignment_start: self.alignment_start,
            read_group: self.read_group_id,
            read_name: self.read_name,
            next_mate_bit_flags: self.next_mate_flags,
            next_fragment_reference_sequence_id: self.next_fragment_reference_sequence_id,
            next_mate_alignment_start: self.next_mate_alignment_start,
            template_size: self.template_size,
            distance_to_next_fragment: self.distance_to_next_fragment,
            tags: self.tags,
            bases: self.bases,
            features: self.features,
            mapping_quality: self.mapping_quality,
            quality_scores: self.quality_scores,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let builder = Builder::default();

        assert_eq!(builder.id, 0);
        assert_eq!(builder.bam_flags, sam::record::Flags::UNMAPPED);
        assert_eq!(builder.flags, Flags::default());
        assert!(builder.reference_sequence_id.is_none());
        assert_eq!(builder.read_length, 0);
        assert_eq!(builder.alignment_start, 0);
        assert_eq!(builder.read_group_id, ReadGroupId::default());
        assert!(builder.read_name.is_empty());
        assert_eq!(builder.next_mate_flags, NextMateFlags::default());
        assert!(builder.next_fragment_reference_sequence_id.is_none());
        assert_eq!(builder.next_mate_alignment_start, 0);
        assert_eq!(builder.template_size, 0);
        assert_eq!(builder.distance_to_next_fragment, 0);
        assert!(builder.tags.is_empty());
        assert!(builder.bases.is_empty());
        assert!(builder.features.is_empty());
        assert_eq!(
            builder.mapping_quality,
            sam::record::MappingQuality::default()
        );
        assert!(builder.quality_scores.is_empty());
    }
}
