/// An enum describing the possible errors that can occur when mapping a sequence to a target feature.
#[derive(Debug, Clone, Copy)]
pub enum MappingError {
    // The anchor sequence is missing - used in `CrisprMapper`.
    MissingAnchor,
    // The protospacer sequence is missing - used in `CrisprMapper`.
    MissingProtospacer,
    // The probe sequence is missing - used in `ProbeMapper` - can be an error for all probe-based mappers.
    MissingProbe,
    // The flex sequence is missing - used in `FlexMapper`.
    MissingFlexSequence,
    // The generic target sequence is missing - used in `GenericMapper`.
    MissingTargetSequence,
}
