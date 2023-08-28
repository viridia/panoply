/// Represents a predicate which can be used to conditionally style a node.
/// Selectors support a subset of CSS grammar:
///
/// ```
///   &
///   &.name
///   .state > &
///   .state > * > &.name
/// ```
///
///
pub struct Selector {}
