//! Lens core primitives.
//!
//! ## Sprint Plan
//! 1. Core identifiers and event envelope. Tests: constructor, default state, display, and builder coverage.
//! 2. Flow and message ID newtypes. Tests: equality, hashing, and monotonic formatting.
//! 3. Direction, severity, and sensitivity enums. Tests: display forms and round-trip matching.
//! 4. Session identity model. Tests: session creation and association with flows.
//! 5. Time abstraction. Tests: fixed clock behavior and timestamp formatting.
//! 6. Core error enum. Tests: display text and classification mapping.
//! 7. CLI skeleton. Tests: argument parsing and default subcommand behavior.
//! 8. Config precedence resolver. Tests: flags over env over file over defaults.
//! 9. Help output contract. Tests: stable help text and examples.
//! 10. Doctor diagnostics. Tests: platform and trust-state reporting.
//! 11. Explicit proxy bind path. Tests: bind address validation and listener setup.
//! 12. Connection accept loop. Tests: accept lifecycle and teardown.
//! 13. Upstream resolution. Tests: config, SNI, and transparent lookup fixtures.
//! 14. Timeout policy. Tests: per-phase expiration and grace period handling.
//! 15. TLS CA generation primitives. Tests: key/cert metadata and validity invariants.
//! 16. Leaf certificate cache. Tests: cache hit, miss, and eviction behavior.
//! 17. Trust-store adapter. Tests: install, uninstall, and error propagation.
//! 18. HTTP/1.1 request framing. Tests: fragmented request corpus.
//! 19. HTTP/1.1 response framing. Tests: pipelined response corpus.
//! 20. Body truncation and buffer reuse. Tests: caps and reuse accounting.
//! 21. Backpressure counters. Tests: queue saturation and drop visibility.
//! 22. Graceful shutdown. Tests: drain, cancel, and bounded exit timing.
//! 23. Store actor. Tests: single-writer ordering and bounded retention.
//! 24. Flow indexing. Tests: lookup consistency and eviction semantics.
//! 25. Snapshot export. Tests: stable structured output fixtures.
//! 26. TUI shell. Tests: render smoke and state transitions.
//! 27. Flow map view. Tests: layout snapshot and label stability.
//! 28. Inspector view. Tests: field ordering and redacted/reveal states.
//! 29. Redaction engine. Tests: secret masking fixtures and rule coverage.
//! 30. PostgreSQL decoder. Tests: golden wire-protocol corpus.
//! 31. Protocol registry. Tests: detector selection and fallback behavior.
//! 32. Replay reader. Tests: captured artifact round-trip.
//! 33. Export formats. Tests: JSONL, JSON, and HAR goldens.
//! 34. Plugin ABI metadata. Tests: compatibility and version checks.
//! 35. Plugin sandbox host. Tests: fuel, memory, and host-call limits.
//! 36. Optional Linux discovery hook. Tests: feature gating and Linux-only behavior.
//! 37. Fuzz targets. Tests: smoke corpus and panic-free mutation runs.
//! 38. Stress harness. Tests: many connections, long bodies, and shutdown under load.
//! 39. Performance harness. Tests: latency, throughput, memory, and allocation regressions.
//! 40. Release validation and docs sync. Tests: workspace, docs, and release checklist.
//!
//! Milestone 1 in this commit implements the shared event envelope and identifier primitives.

use std::fmt;

/// Session identifier within a run.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(u64);

impl SessionId {
    /// Creates a new session identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Flow identifier within a run.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FlowId(u64);

impl FlowId {
    /// Creates a new flow identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier within a flow.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId(u64);

impl MessageId {
    /// Creates a new message identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Run identifier for a capture session.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunId(u64);

impl RunId {
    /// Creates a new run identifier.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Version of the core event schema.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaVersion {
    /// Major schema version.
    pub major: u16,
    /// Minor schema version.
    pub minor: u16,
}

impl SchemaVersion {
    /// Current schema version for milestone 1.
    pub const CURRENT: Self = Self { major: 0, minor: 1 };

    /// Creates a new schema version.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Traffic direction relative to the client.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    /// Client to server.
    ClientToServer,
    /// Server to client.
    ServerToClient,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ClientToServer => "client_to_server",
            Self::ServerToClient => "server_to_client",
        })
    }
}

/// Log or event severity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// Sensitivity classification for captured data.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sensitivity {
    /// Public data.
    Public,
    /// Data has been redacted.
    Redacted,
    /// Data contains secrets or plaintext payloads.
    Secret,
}

impl fmt::Display for Sensitivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Public => "public",
            Self::Redacted => "redacted",
            Self::Secret => "secret",
        })
    }
}

/// Origin of an event in the pipeline.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventSource {
    /// Proxy accept/forwarding path.
    Proxy,
    /// TLS certificate and handshake path.
    Tls,
    /// Protocol detection or decoding path.
    Decoder,
    /// Store and aggregation path.
    Store,
    /// User interface path.
    Ui,
    /// CLI and command path.
    Cli,
    /// Plugin host path.
    Plugin,
    /// Benchmark or stress harness path.
    Benchmark,
}

impl fmt::Display for EventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Proxy => "proxy",
            Self::Tls => "tls",
            Self::Decoder => "decoder",
            Self::Store => "store",
            Self::Ui => "ui",
            Self::Cli => "cli",
            Self::Plugin => "plugin",
            Self::Benchmark => "benchmark",
        })
    }
}

/// Shared immutable event envelope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventEnvelope {
    /// Semantic event name.
    pub event_type: String,
    /// Schema version for the payload.
    pub schema_version: SchemaVersion,
    /// Run identifier.
    pub run_id: RunId,
    /// Session identifier, when relevant.
    pub session_id: Option<SessionId>,
    /// Flow identifier, when relevant.
    pub flow_id: Option<FlowId>,
    /// Message identifier, when relevant.
    pub message_id: Option<MessageId>,
    /// Traffic direction, when relevant.
    pub direction: Option<Direction>,
    /// Monotonic timestamp in nanoseconds.
    pub ts_mono_nanos: u64,
    /// Wall-clock timestamp in nanoseconds.
    pub ts_wall_nanos: u64,
    /// Originating subsystem.
    pub source: EventSource,
    /// Severity classification.
    pub severity: Severity,
    /// Sensitivity classification.
    pub sensitivity: Sensitivity,
}

impl EventEnvelope {
    /// Creates a new default-safe envelope for a given event type.
    #[must_use]
    pub fn new(event_type: impl Into<String>, run_id: RunId, source: EventSource) -> Self {
        Self {
            event_type: event_type.into(),
            schema_version: SchemaVersion::CURRENT,
            run_id,
            session_id: None,
            flow_id: None,
            message_id: None,
            direction: None,
            ts_mono_nanos: 0,
            ts_wall_nanos: 0,
            source,
            severity: Severity::Info,
            sensitivity: Sensitivity::Public,
        }
    }

    /// Attaches a session identifier.
    #[must_use]
    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Attaches a flow identifier.
    #[must_use]
    pub fn with_flow_id(mut self, flow_id: FlowId) -> Self {
        self.flow_id = Some(flow_id);
        self
    }

    /// Attaches a message identifier.
    #[must_use]
    pub fn with_message_id(mut self, message_id: MessageId) -> Self {
        self.message_id = Some(message_id);
        self
    }

    /// Attaches a direction.
    #[must_use]
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Attaches both monotonic and wall-clock timestamps.
    #[must_use]
    pub fn with_timestamps(mut self, ts_mono_nanos: u64, ts_wall_nanos: u64) -> Self {
        self.ts_mono_nanos = ts_mono_nanos;
        self.ts_wall_nanos = ts_wall_nanos;
        self
    }

    /// Updates severity.
    #[must_use]
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Updates sensitivity.
    #[must_use]
    pub fn with_sensitivity(mut self, sensitivity: Sensitivity) -> Self {
        self.sensitivity = sensitivity;
        self
    }
}

impl Default for EventEnvelope {
    fn default() -> Self {
        Self::new("event.unknown", RunId::new(0), EventSource::Cli)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_defaults_are_safe_and_stable() {
        let envelope = EventEnvelope::new("flow.opened", RunId::new(7), EventSource::Proxy);

        assert_eq!(envelope.event_type, "flow.opened");
        assert_eq!(envelope.schema_version, SchemaVersion::CURRENT);
        assert_eq!(envelope.run_id, RunId::new(7));
        assert_eq!(envelope.session_id, None);
        assert_eq!(envelope.flow_id, None);
        assert_eq!(envelope.message_id, None);
        assert_eq!(envelope.direction, None);
        assert_eq!(envelope.ts_mono_nanos, 0);
        assert_eq!(envelope.ts_wall_nanos, 0);
        assert_eq!(envelope.source, EventSource::Proxy);
        assert_eq!(envelope.severity, Severity::Info);
        assert_eq!(envelope.sensitivity, Sensitivity::Public);
    }

    #[test]
    fn envelope_builder_attaches_context() {
        let envelope = EventEnvelope::new("message.emitted", RunId::new(9), EventSource::Decoder)
            .with_session_id(SessionId::new(11))
            .with_flow_id(FlowId::new(13))
            .with_message_id(MessageId::new(17))
            .with_direction(Direction::ClientToServer)
            .with_timestamps(19, 23)
            .with_severity(Severity::Warning)
            .with_sensitivity(Sensitivity::Redacted);

        assert_eq!(envelope.session_id, Some(SessionId::new(11)));
        assert_eq!(envelope.flow_id, Some(FlowId::new(13)));
        assert_eq!(envelope.message_id, Some(MessageId::new(17)));
        assert_eq!(envelope.direction, Some(Direction::ClientToServer));
        assert_eq!(envelope.ts_mono_nanos, 19);
        assert_eq!(envelope.ts_wall_nanos, 23);
        assert_eq!(envelope.severity, Severity::Warning);
        assert_eq!(envelope.sensitivity, Sensitivity::Redacted);
    }

    #[test]
    fn display_forms_are_stable() {
        assert_eq!(SessionId::new(5).to_string(), "5");
        assert_eq!(FlowId::new(6).to_string(), "6");
        assert_eq!(MessageId::new(7).to_string(), "7");
        assert_eq!(RunId::new(8).to_string(), "8");
        assert_eq!(SchemaVersion::new(2, 4).to_string(), "2.4");
        assert_eq!(Direction::ClientToServer.to_string(), "client_to_server");
        assert_eq!(Direction::ServerToClient.to_string(), "server_to_client");
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Sensitivity::Public.to_string(), "public");
        assert_eq!(Sensitivity::Redacted.to_string(), "redacted");
        assert_eq!(Sensitivity::Secret.to_string(), "secret");
        assert_eq!(EventSource::Proxy.to_string(), "proxy");
        assert_eq!(EventSource::Tls.to_string(), "tls");
        assert_eq!(EventSource::Decoder.to_string(), "decoder");
        assert_eq!(EventSource::Store.to_string(), "store");
        assert_eq!(EventSource::Ui.to_string(), "ui");
        assert_eq!(EventSource::Cli.to_string(), "cli");
        assert_eq!(EventSource::Plugin.to_string(), "plugin");
        assert_eq!(EventSource::Benchmark.to_string(), "benchmark");
    }
}
