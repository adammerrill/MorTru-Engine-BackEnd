/// Serialize a typed Rust value into a MISMO 3.4 XML string.
///
/// This is a thin wrapper around `quick_xml::se::to_string` that maps the
/// underlying error into [`crate::MismoError::Parse`] (quick-xml 0.36 uses
/// `DeError` for both parse and serialize failures).
///
/// The output is an XML fragment without an XML declaration header. The
/// root element name is determined by the `#[serde(rename = "...")]`
/// annotation on `T`.
///
/// # Errors
/// Returns [`crate::MismoError::Parse`] if `value` cannot be serialized to
/// XML (e.g. a `None` field marked required in the serde schema).
///
/// # Example
/// ```ignore
/// let xml: String = mismo::xml::serialize::to_xml(&message)?;
/// assert!(xml.starts_with("<MESSAGE>"));
/// ```
pub fn to_xml<T: serde::Serialize>(value: &T) -> crate::Result<String> {
    quick_xml::se::to_string(value).map_err(crate::MismoError::Parse)
}
