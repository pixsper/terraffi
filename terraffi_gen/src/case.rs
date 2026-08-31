//! Naming conventions for generated C identifiers.

/// The naming convention applied to a generated C identifier.
///
/// Only conventions that produce valid C identifiers are offered. Styles such as
/// kebab, title and train introduce hyphens or spaces, which would emit a header
/// that does not compile.
///
/// Defining this type here keeps the case-conversion crate out of terraffi's public
/// API, so a breaking release of it cannot force one here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Case {
    /// `example_name`
    Snake,
    /// `EXAMPLE_NAME`
    UpperSnake,
    /// `ExampleName`
    Pascal,
    /// `exampleName`
    Camel,
    /// `Example_Name`
    Ada,
    /// `examplename`
    Flat,
    /// `EXAMPLENAME`
    UpperFlat,
}

impl Case {
    /// Maps to the underlying conversion crate's representation.
    ///
    /// `#[non_exhaustive]` only constrains downstream crates, so this match stays
    /// exhaustive and a new variant here is a compile error until it is mapped.
    pub(crate) fn to_convert_case(self) -> convert_case::Case<'static> {
        match self {
            Case::Snake => convert_case::Case::Snake,
            Case::UpperSnake => convert_case::Case::UpperSnake,
            Case::Pascal => convert_case::Case::Pascal,
            Case::Camel => convert_case::Case::Camel,
            Case::Ada => convert_case::Case::Ada,
            Case::Flat => convert_case::Case::Flat,
            Case::UpperFlat => convert_case::Case::UpperFlat,
        }
    }
}
