/// XML serialization and deserialization infrastructure.
///
/// These modules wrap `quick-xml` 0.36 with the `serialize` feature and
/// map underlying errors to [`crate::MismoError`].
///
/// Direct use of these functions is an implementation detail of the
/// `schema` module. External callers should use the higher-level
/// [`crate::schema::message::MismoMessage::from_xml`] and
/// [`crate::schema::message::MismoMessage::to_xml`] methods instead.
pub mod parse;
pub mod serialize;
