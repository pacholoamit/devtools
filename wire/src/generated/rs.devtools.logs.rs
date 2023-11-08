#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Update {
    /// A list of log events that happened since the last update.
    #[prost(message, repeated, tag = "1")]
    pub log_events: ::prost::alloc::vec::Vec<LogEvent>,
    /// A count of how many log events were dropped because
    /// the event buffer was at capacity.
    ///
    /// If everything is working correctly, this should be 0. If this
    /// number is greater than zero this indicates the event buffers capacity
    /// should be increased or the publish interval decreased.
    #[prost(uint64, tag = "2")]
    pub dropped_events: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogEvent {
    /// The main message body of the log.
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
    /// Log events can happen inside of spans and if they do, this field will indicate which span it was.
    #[prost(uint64, optional, tag = "2")]
    pub parent: ::core::option::Option<u64>,
    /// Identifier for metadata describing static characteristics of all spans originating
    /// from that call site, such as its name, source code location, verbosity level, and
    /// the names of its fields.
    #[prost(uint64, tag = "3")]
    pub metadata_id: u64,
    /// User-defined key-value pairs of arbitrary data associated with the event.
    #[prost(message, repeated, tag = "4")]
    pub fields: ::prost::alloc::vec::Vec<super::common::Field>,
    /// Timestamp for the log event.
    #[prost(message, optional, tag = "5")]
    pub at: ::core::option::Option<::prost_types::Timestamp>,
}
