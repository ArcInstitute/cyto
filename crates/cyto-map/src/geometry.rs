use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Which read of the pair a component appears in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ReadMate {
    R1,
    R2,
}

/// Functional components that can appear in a sequencing read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    Barcode,
    Umi,
    Probe,
    Anchor,
    Protospacer,
    Gex,
}

impl Component {
    /// Returns true if this component requires an explicit length in the geometry spec.
    pub fn requires_length(&self) -> bool {
        matches!(self, Component::Umi)
    }

    /// Returns true if this component forbids an explicit length (inferred from references).
    pub fn forbids_length(&self) -> bool {
        !self.requires_length()
    }
}

impl FromStr for Component {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "barcode" | "bc" => Ok(Component::Barcode),
            "umi" => Ok(Component::Umi),
            "probe" => Ok(Component::Probe),
            "anchor" => Ok(Component::Anchor),
            "protospacer" => Ok(Component::Protospacer),
            "gex" => Ok(Component::Gex),
            _ => Err(ParseError::UnknownComponent(s.to_string())),
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Component::Barcode => write!(f, "barcode"),
            Component::Umi => write!(f, "umi"),
            Component::Probe => write!(f, "probe"),
            Component::Anchor => write!(f, "anchor"),
            Component::Protospacer => write!(f, "protospacer"),
            Component::Gex => write!(f, "gex"),
        }
    }
}

/// A region within a read: either a known component or an anonymous skip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Region {
    Component {
        kind: Component,
        length: Option<usize>,
    },
    Skip {
        length: usize,
    },
}

/// A single read (R1 or R2) consisting of ordered regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    pub regions: Vec<Region>,
}

/// Complete geometry specification for paired-end reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub r1: Read,
    pub r2: Read,
}

/// Errors that can occur when parsing a geometry string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingSeparator,
    MultipleSeparators,
    EmptyRead { read: &'static str },
    UnclosedBracket { position: usize },
    EmptyBracket { position: usize },
    UnknownComponent(String),
    MissingRequiredLength { component: Component },
    UnexpectedLength { component: Component },
    InvalidLength { value: String },
    ZeroLengthSkip { position: usize },
    DuplicateComponent { component: Component },
    UnexpectedChar { char: char, position: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingSeparator => {
                write!(f, "missing '|' separator between R1 and R2")
            }
            ParseError::MultipleSeparators => {
                write!(f, "multiple '|' separators found, expected exactly one")
            }
            ParseError::EmptyRead { read } => {
                write!(f, "{read} is empty, expected at least one region")
            }
            ParseError::UnclosedBracket { position } => {
                write!(f, "unclosed bracket starting at position {position}")
            }
            ParseError::EmptyBracket { position } => {
                write!(f, "empty bracket at position {position}")
            }
            ParseError::UnknownComponent(name) => {
                write!(f, "unknown component '{name}'")
            }
            ParseError::MissingRequiredLength { component } => {
                write!(
                    f,
                    "[{component}] requires an explicit length, use [{component}:N]"
                )
            }
            ParseError::UnexpectedLength { component } => {
                write!(
                    f,
                    "[{component}] cannot have an explicit length (inferred from reference)"
                )
            }
            ParseError::InvalidLength { value } => {
                write!(f, "invalid length '{value}', expected positive integer")
            }
            ParseError::ZeroLengthSkip { position } => {
                write!(f, "skip at position {position} has zero length")
            }
            ParseError::DuplicateComponent { component } => {
                write!(f, "component [{component}] appears multiple times")
            }
            ParseError::UnexpectedChar { char, position } => {
                write!(f, "unexpected character '{char}' at position {position}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Geometry {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_geometry(s)
    }
}

fn parse_geometry(input: &str) -> Result<Geometry, ParseError> {
    let parts: Vec<&str> = input.split('|').collect();

    match parts.len() {
        1 => return Err(ParseError::MissingSeparator),
        2 => {}
        _ => return Err(ParseError::MultipleSeparators),
    }

    let r1 = parse_read(parts[0], "R1")?;
    let r2 = parse_read(parts[1], "R2")?;

    // Check for duplicate components across both reads
    let mut seen = std::collections::HashSet::new();
    for region in r1.regions.iter().chain(r2.regions.iter()) {
        if let Region::Component { kind, .. } = region
            && !seen.insert(*kind)
        {
            return Err(ParseError::DuplicateComponent { component: *kind });
        }
    }

    Ok(Geometry { r1, r2 })
}

fn parse_read(input: &str, read_name: &'static str) -> Result<Read, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::EmptyRead { read: read_name });
    }

    let mut regions = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        match ch {
            '[' => {
                let start = pos;
                let mut content = String::new();

                loop {
                    match chars.next() {
                        Some((_, ']')) => break,
                        Some((_, c)) => content.push(c),
                        None => return Err(ParseError::UnclosedBracket { position: start }),
                    }
                }

                let region = parse_region(&content, start)?;
                regions.push(region);
            }
            ' ' | '\t' => continue,
            _ => {
                return Err(ParseError::UnexpectedChar {
                    char: ch,
                    position: pos,
                });
            }
        }
    }

    if regions.is_empty() {
        return Err(ParseError::EmptyRead { read: read_name });
    }

    Ok(Read { regions })
}

fn parse_region(content: &str, position: usize) -> Result<Region, ParseError> {
    let content = content.trim();

    if content.is_empty() {
        return Err(ParseError::EmptyBracket { position });
    }

    if let Some(len_str) = content.strip_prefix(':') {
        let length = parse_length(len_str)?;

        if length == 0 {
            return Err(ParseError::ZeroLengthSkip { position });
        }

        return Ok(Region::Skip { length });
    }

    let (name, length) = match content.split_once(':') {
        Some((n, l)) => (n.trim(), Some(parse_length(l.trim())?)),
        None => (content, None),
    };

    let kind: Component = name.parse()?;

    if kind.requires_length() && length.is_none() {
        return Err(ParseError::MissingRequiredLength { component: kind });
    }

    if kind.forbids_length() && length.is_some() {
        return Err(ParseError::UnexpectedLength { component: kind });
    }

    Ok(Region::Component { kind, length })
}

fn parse_length(s: &str) -> Result<usize, ParseError> {
    s.parse().map_err(|_| ParseError::InvalidLength {
        value: s.to_string(),
    })
}

// --- Resolution ---

/// Resolved position information for a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRegion {
    pub offset: usize,
    pub length: Option<usize>,
    pub mate: ReadMate,
}

/// Fully resolved geometry with concrete positions.
#[derive(Debug, Clone)]
pub struct ResolvedGeometry {
    components: HashMap<Component, ResolvedRegion>,
    pub r1_length: Option<usize>,
    pub r2_length: Option<usize>,
}

impl ResolvedGeometry {
    /// Get the resolved region for a component.
    pub fn get(&self, component: Component) -> Option<&ResolvedRegion> {
        self.components.get(&component)
    }

    /// Get the offset of a component.
    pub fn offset(&self, component: Component) -> Option<usize> {
        self.get(component).map(|r| r.offset)
    }

    /// Get the mate (R1/R2) of a component.
    pub fn mate(&self, component: Component) -> Option<ReadMate> {
        self.get(component).map(|r| r.mate)
    }

    pub fn get_expected_length(&self, component: Component) -> Result<usize, ResolveError> {
        if let Some(region) = self.get(component) {
            if let Some(length) = region.length {
                Ok(length)
            } else {
                Err(ResolveError::MissingLength { component })
            }
        } else {
            Err(ResolveError::MissingLength { component })
        }
    }

    /// Get the length of the barcode.
    pub fn get_barcode_length(&self) -> Result<usize, ResolveError> {
        self.get_expected_length(Component::Barcode)
    }

    /// Get the length of the umi.
    pub fn get_umi_length(&self) -> Result<usize, ResolveError> {
        self.get_expected_length(Component::Umi)
    }
}

/// Errors that can occur during geometry resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    MissingLength { component: Component },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::MissingLength { component } => {
                write!(f, "could not determine length for [{component}]")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

impl Geometry {
    /// Resolve the geometry to concrete positions using a length provider.
    ///
    /// The `length_fn` is called for each component to get its sequence length.
    /// It should return `None` for variable-length components (e.g., anchor).
    pub fn resolve<F>(&self, length_fn: F) -> Result<ResolvedGeometry, ResolveError>
    where
        F: Fn(Component) -> Option<usize>,
    {
        let mut components = HashMap::new();

        let (r1_components, r1_length) = resolve_read(&self.r1, &length_fn, ReadMate::R1)?;
        let (r2_components, r2_length) = resolve_read(&self.r2, &length_fn, ReadMate::R2)?;

        components.extend(r1_components);
        components.extend(r2_components);

        Ok(ResolvedGeometry {
            components,
            r1_length,
            r2_length,
        })
    }
}

fn resolve_read<F>(
    read: &Read,
    length_fn: &F,
    mate: ReadMate,
) -> Result<(HashMap<Component, ResolvedRegion>, Option<usize>), ResolveError>
where
    F: Fn(Component) -> Option<usize>,
{
    let mut result = HashMap::new();
    let mut offset = 0usize;
    let mut total_length: Option<usize> = Some(0);

    for region in &read.regions {
        match region {
            Region::Skip { length } => {
                offset += length;
                if let Some(ref mut total) = total_length {
                    *total += length;
                }
            }
            Region::Component {
                kind,
                length: explicit_len,
            } => {
                let len = if let Some(l) = explicit_len {
                    Some(*l)
                } else {
                    length_fn(*kind)
                };

                if let Some(l) = len {
                    result.insert(
                        *kind,
                        ResolvedRegion {
                            offset,
                            length: Some(l),
                            mate,
                        },
                    );
                    offset += l;
                    if let Some(ref mut total) = total_length {
                        *total += l;
                    }
                } else {
                    // Variable-length component
                    result.insert(
                        *kind,
                        ResolvedRegion {
                            offset,
                            length: None,
                            mate,
                        },
                    );
                    total_length = None;
                }
            }
        }
    }

    Ok((result, total_length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_gex() {
        let geo: Geometry = "[barcode][umi:12]|[gex]".parse().unwrap();

        assert_eq!(geo.r1.regions.len(), 2);
        assert_eq!(geo.r2.regions.len(), 1);

        assert!(matches!(
            &geo.r1.regions[0],
            Region::Component {
                kind: Component::Barcode,
                length: None
            }
        ));
        assert!(matches!(
            &geo.r1.regions[1],
            Region::Component {
                kind: Component::Umi,
                length: Some(12)
            }
        ));
        assert!(matches!(
            &geo.r2.regions[0],
            Region::Component {
                kind: Component::Gex,
                length: None
            }
        ));
    }

    #[test]
    fn test_gex_with_probe() {
        let geo: Geometry = "[barcode][umi:12]|[gex][:18][probe]".parse().unwrap();

        assert_eq!(geo.r2.regions.len(), 3);
        assert!(matches!(&geo.r2.regions[1], Region::Skip { length: 18 }));
        assert!(matches!(
            &geo.r2.regions[2],
            Region::Component {
                kind: Component::Probe,
                length: None
            }
        ));
    }

    #[test]
    fn test_crispr() {
        let geo: Geometry = "[barcode][umi:12]|[:20][probe][:6][anchor][protospacer]"
            .parse()
            .unwrap();

        assert_eq!(geo.r2.regions.len(), 5);
        assert!(matches!(&geo.r2.regions[0], Region::Skip { length: 20 }));
        assert!(matches!(
            &geo.r2.regions[1],
            Region::Component {
                kind: Component::Probe,
                length: None
            }
        ));
        assert!(matches!(&geo.r2.regions[2], Region::Skip { length: 6 }));
        assert!(matches!(
            &geo.r2.regions[3],
            Region::Component {
                kind: Component::Anchor,
                length: None
            }
        ));
        assert!(matches!(
            &geo.r2.regions[4],
            Region::Component {
                kind: Component::Protospacer,
                length: None
            }
        ));
    }

    #[test]
    fn test_barcode_alias() {
        let geo: Geometry = "[bc][umi:12]|[gex]".parse().unwrap();
        assert!(matches!(
            &geo.r1.regions[0],
            Region::Component {
                kind: Component::Barcode,
                length: None
            }
        ));
    }

    #[test]
    fn test_whitespace_tolerance() {
        let geo: Geometry = "  [barcode] [umi:12]  |  [gex]  ".parse().unwrap();
        assert_eq!(geo.r1.regions.len(), 2);
        assert_eq!(geo.r2.regions.len(), 1);
    }

    #[test]
    fn test_error_missing_separator() {
        let err = "[barcode][umi:12][gex]".parse::<Geometry>().unwrap_err();
        assert!(matches!(err, ParseError::MissingSeparator));
    }

    #[test]
    fn test_error_multiple_separators() {
        let err = "[barcode]|[umi:12]|[gex]".parse::<Geometry>().unwrap_err();
        assert!(matches!(err, ParseError::MultipleSeparators));
    }

    #[test]
    fn test_error_umi_requires_length() {
        let err = "[barcode][umi]|[gex]".parse::<Geometry>().unwrap_err();
        assert!(matches!(
            err,
            ParseError::MissingRequiredLength {
                component: Component::Umi
            }
        ));
    }

    #[test]
    fn test_error_barcode_forbids_length() {
        let err = "[barcode:16][umi:12]|[gex]"
            .parse::<Geometry>()
            .unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedLength {
                component: Component::Barcode
            }
        ));
    }

    #[test]
    fn test_error_unknown_component() {
        let err = "[barcode][umi:12]|[unknown]"
            .parse::<Geometry>()
            .unwrap_err();
        assert!(matches!(err, ParseError::UnknownComponent(_)));
    }

    #[test]
    fn test_error_duplicate_component() {
        let err = "[barcode][umi:12]|[barcode]"
            .parse::<Geometry>()
            .unwrap_err();
        assert!(matches!(
            err,
            ParseError::DuplicateComponent {
                component: Component::Barcode
            }
        ));
    }

    #[test]
    fn test_error_unclosed_bracket() {
        let err = "[barcode][umi:12|[gex]".parse::<Geometry>().unwrap_err();
        assert!(matches!(err, ParseError::UnclosedBracket { .. }));
    }

    #[test]
    fn test_error_empty_bracket() {
        let err = "[barcode][][umi:12]|[gex]".parse::<Geometry>().unwrap_err();
        assert!(matches!(err, ParseError::EmptyBracket { .. }));
    }

    #[test]
    fn test_error_zero_length_skip() {
        let err = "[barcode][umi:12]|[:0][gex]"
            .parse::<Geometry>()
            .unwrap_err();
        assert!(matches!(err, ParseError::ZeroLengthSkip { .. }));
    }

    #[test]
    fn test_error_empty_read() {
        let err = "[barcode][umi:12]|".parse::<Geometry>().unwrap_err();
        assert!(matches!(err, ParseError::EmptyRead { read: "R2" }));
    }

    // Resolution tests

    fn test_lengths(component: Component) -> Option<usize> {
        match component {
            Component::Barcode => Some(16),
            Component::Umi => None,
            Component::Probe => Some(8),
            Component::Anchor => None,
            Component::Protospacer => Some(20),
            Component::Gex => Some(25),
        }
    }

    #[test]
    fn test_resolve_simple_gex() {
        let geo: Geometry = "[barcode][umi:12]|[gex]".parse().unwrap();
        let resolved = geo.resolve(test_lengths).unwrap();

        assert_eq!(resolved.offset(Component::Barcode), Some(0));
        assert_eq!(resolved.offset(Component::Umi), Some(16));
        assert_eq!(resolved.mate(Component::Barcode), Some(ReadMate::R1));
        assert_eq!(resolved.mate(Component::Umi), Some(ReadMate::R1));
        assert_eq!(resolved.r1_length, Some(28));

        assert_eq!(resolved.offset(Component::Gex), Some(0));
        assert_eq!(resolved.mate(Component::Gex), Some(ReadMate::R2));
        assert_eq!(resolved.r2_length, Some(25));
    }

    #[test]
    fn test_resolve_gex_with_probe() {
        let geo: Geometry = "[barcode][umi:12]|[gex][:18][probe]".parse().unwrap();
        let resolved = geo.resolve(test_lengths).unwrap();

        assert_eq!(resolved.offset(Component::Gex), Some(0));
        assert_eq!(resolved.offset(Component::Probe), Some(43));
        assert_eq!(resolved.mate(Component::Probe), Some(ReadMate::R2));
        assert_eq!(resolved.r2_length, Some(51));
    }

    #[test]
    fn test_resolve_crispr() {
        let geo: Geometry = "[barcode][umi:12]|[:20][probe][:6][anchor][protospacer]"
            .parse()
            .unwrap();
        let resolved = geo.resolve(test_lengths).unwrap();

        assert_eq!(resolved.offset(Component::Barcode), Some(0));
        assert_eq!(resolved.offset(Component::Umi), Some(16));

        assert_eq!(resolved.offset(Component::Probe), Some(20));
        assert_eq!(resolved.offset(Component::Anchor), Some(34));
        assert_eq!(resolved.mate(Component::Anchor), Some(ReadMate::R2));

        // R2 length indeterminate due to variable anchor
        assert_eq!(resolved.r2_length, None);
    }

    #[test]
    fn test_resolve_probe_on_r1() {
        let geo: Geometry = "[barcode][umi:12][:10][probe]|[:20][anchor][protospacer]"
            .parse()
            .unwrap();
        let resolved = geo.resolve(test_lengths).unwrap();

        assert_eq!(resolved.offset(Component::Probe), Some(38));
        assert_eq!(resolved.mate(Component::Probe), Some(ReadMate::R1));

        assert_eq!(resolved.offset(Component::Anchor), Some(20));
        assert_eq!(resolved.mate(Component::Anchor), Some(ReadMate::R2));
    }
}
