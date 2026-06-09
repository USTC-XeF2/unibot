use crate::models::MessageSegment;
use crate::protocol::types::MilkySegment;

/// Convert a single internal `MessageSegment` to `MilkySegment`.
pub fn internal_to_milky_segment(seg: &MessageSegment) -> MilkySegment {
    match seg {
        MessageSegment::Text { text } => MilkySegment::Text { text: text.clone() },
        MessageSegment::Image { file, url } => MilkySegment::Image {
            file: file.clone(),
            url: url.clone(),
        },
        MessageSegment::At { target } => MilkySegment::At { qq: target.clone() },
        MessageSegment::AtAll => MilkySegment::AtAll {},
        MessageSegment::Face { id } => MilkySegment::Face { id: id.clone() },
    }
}

/// Convert a single `MilkySegment` to internal `MessageSegment`.
pub fn milky_to_internal_segment(seg: &MilkySegment) -> MessageSegment {
    match seg {
        MilkySegment::Text { text } => MessageSegment::Text { text: text.clone() },
        MilkySegment::Image { file, url } => MessageSegment::Image {
            file: file.clone(),
            url: url.clone(),
        },
        MilkySegment::At { qq } => MessageSegment::At { target: qq.clone() },
        MilkySegment::AtAll {} => MessageSegment::AtAll,
        MilkySegment::Face { id } => MessageSegment::Face { id: id.clone() },
    }
}

/// Convert a slice of internal `MessageSegment`s to `Vec<MilkySegment>`.
pub fn internal_to_milky_segments(segments: &[MessageSegment]) -> Vec<MilkySegment> {
    segments.iter().map(internal_to_milky_segment).collect()
}

/// Convert a slice of `MilkySegment`s to `Vec<MessageSegment>`.
pub fn milky_to_internal_segments(segments: &[MilkySegment]) -> Vec<MessageSegment> {
    segments.iter().map(milky_to_internal_segment).collect()
}
