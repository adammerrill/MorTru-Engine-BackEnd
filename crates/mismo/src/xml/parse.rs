/// Deserialize a MISMO 3.4 XML document into a typed Rust value.
///
/// This is a thin wrapper around `quick_xml::de::from_str` that maps the
/// underlying parse error into [`crate::MismoError::Parse`].
///
/// The input must be a valid UTF-8 XML document. The top-level element name
/// must match the `#[serde(rename = "...")]` annotation on `T`.
///
/// # Errors
/// Returns [`crate::MismoError::Parse`] if `xml` is not valid XML or if
/// the structure does not match the serde schema for `T`.
///
/// # Example
/// ```ignore
/// #[derive(serde::Deserialize)]
/// #[serde(rename = "MESSAGE")]
/// struct Message { /* ... */ }
///
/// let msg: Message = mismo::xml::parse::from_xml(xml_str)?;
/// ```
pub fn from_xml<T: serde::de::DeserializeOwned>(xml: &str) -> crate::Result<T> {
    quick_xml::de::from_str(xml).map_err(crate::MismoError::Parse)
}
