/// An enum describing the possible errors that can occur when mapping a sequence to a target feature.
#[derive(Debug, Clone, Copy)]
pub enum MappingError {
    /// The anchor sequence is missing - used in `CrisprMapper`.
    MissingAnchor,
    /// The protospacer sequence is missing - used in `CrisprMapper`.
    MissingProtospacer,
    /// The probe sequence is missing - used in `ProbeMapper` - can be an error for all probe-based mappers.
    MissingProbe,
    /// The gex sequence is missing - used in `GexMapper`.
    MissingGexSequence,
    /// The generic target sequence is missing - used in `GenericMapper`.
    MissingTargetSequence,
    /// The sequence is unexpectedly truncated - used in all mappers.
    UnexpectedlyTruncated,
}
