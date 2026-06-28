# Internal Event Model

## Purpose
This document defines every internal event used by Lens. Each event is described as a contract, not an implementation detail.

The design goal is to keep events small, versioned, thread-safe, and cheap to move between the proxy, decoder, store, UI, and export layers.

## Event Design Principles
- Events are immutable once created.
- Events carry only the data needed by downstream consumers.
- Payload bodies should be shared by reference rather than copied whenever possible.
- All events must be safe to send across threads, even if their payloads are large.
- Every event must be serializable into a stable external representation for storage, export, or debugging.
- Versioned event envelopes are preferred over ad hoc schema drift.

### Trade-offs
- Immutable events simplify concurrency, but they may create more allocations than a mutable in-place model.
- Richer events improve diagnostics, but they increase serialization and versioning work.
- A stable event contract is worth the extra discipline because the UI, storage, and replay paths all depend on it.

## Common Event Envelope
Every internal event shares a common envelope.

### Envelope fields
- `event_id`: unique monotonic identifier within a run
- `event_type`: semantic type name
- `schema_version`: version of the event payload schema
- `run_id`: identifier for the current capture session
- `session_id`: identifier for the connection or flow session when relevant
- `flow_id`: identifier for the flow when relevant
- `direction`: client-to-server or server-to-client when relevant
- `ts_mono`: monotonic timestamp
- `ts_wall`: wall-clock timestamp
- `source`: originating subsystem, such as proxy, TLS, decoder, store, or UI
- `severity`: informational, warning, or error where applicable
- `sensitivity`: public, redacted, or secret-bearing

### Envelope ownership
- The creator owns the envelope until the event is handed to the next stage.
- After emission, the event becomes immutable and may be shared freely.

### Envelope serialization
- Serialize as a versioned record with explicit field names.
- Preserve `schema_version` and `event_type` in all wire and log representations.
- Omit implementation-only pointers or runtime handles.

### Envelope thread safety
- The envelope must be `Send + Sync` in Rust terms or equivalent in any future host language.
- No mutable interior references may be exposed to consumers.

### Envelope lifetime
- The envelope lives for the duration of the event's use in the pipeline.
- In storage, the envelope persists as long as the flow or record is retained.

### Envelope storage policy
- Keep the envelope in memory while the event is active.
- Persist only when the event is part of a flow snapshot, export file, or replay artifact.
- Redacted envelopes may be stored longer than raw payload-bearing variants.

### Envelope versioning
- Schema versioning is mandatory.
- Backward-compatible additions should bump the minor version.
- Breaking field changes should create a new major schema version or a new event type.

## Event Catalog

### 1. `SessionAccepted`
**Purpose**
Record that a new inbound connection has been accepted by the proxy.

**Fields**
- envelope fields
- `listener_addr`
- `client_addr`
- `accept_mode` (`explicit`, `transparent`, `tunnel`)
- `transport` (`tcp`, `unix`, `tls`, `quic` when supported later)

**Ownership**
- Owned by the proxy accept loop until emitted.
- Shared read-only after emission.

**Serialization**
- JSON and binary forms should carry listener and client addresses as structured values, not raw strings only.

**Thread safety**
- Must be safe to pass from the accept task to the session task and store task.

**Lifetime**
- Short-lived event, typically emitted once per connection.

**Storage policy**
- Stored in the flow history and session index.
- Never evicted before derived flow summaries if the session is still active.

**Versioning**
- Additive fields such as proxy protocol metadata should not break existing readers.

**Trade-off**
- Capturing this event gives excellent session tracing, but it adds one more event on every connection.

### 2. `SessionClassified`
**Purpose**
Record how the proxy decided to handle the connection.

**Fields**
- envelope fields
- `classification` (`inspect`, `pass_through`, `tunnel`, `deny`)
- `reason`
- `target_hint`
- `policy_id` when a policy engine is used

**Ownership**
- Owned by the classification stage.

**Serialization**
- Serialize the reason as a stable enum plus optional human text.

**Thread safety**
- Safe to move across the session pipeline and into diagnostics.

**Lifetime**
- Emitted once after classification.

**Storage policy**
- Persist for session audit and doctor-style diagnostics.

**Versioning**
- New classification outcomes must be added as explicit enum variants.

**Trade-off**
- Clear classification makes the engine easier to reason about, but it exposes policy decisions in more places.

### 3. `UpstreamResolved`
**Purpose**
Record how the proxy resolved the destination upstream target.

**Fields**
- envelope fields
- `resolved_host`
- `resolved_port`
- `resolved_ip`
- `resolution_source` (`config`, `sni`, `connect_target`, `transparent_lookup`)
- `resolution_latency_ms`

**Ownership**
- Owned by the resolution stage.

**Serialization**
- Preserve the original hint and the resolved target separately.

**Thread safety**
- Resolution results are read-only once emitted.

**Lifetime**
- Usually emitted once per session, but can be re-emitted on reconnect.

**Storage policy**
- Store with the flow so replay and export can see what was actually contacted.

**Versioning**
- Future resolution sources should be added without changing the meaning of existing ones.

**Trade-off**
- This event improves observability for transparent mode, but it creates extra metadata that may be absent in tunnel mode.

### 4. `CertificateRequested`
**Purpose**
Record that the TLS subsystem needs a leaf certificate for a hostname.

**Fields**
- envelope fields
- `hostname`
- `requested_san_names`
- `certificate_profile`
- `cache_hit`
- `cache_key`

**Ownership**
- Owned by the TLS certificate manager.

**Serialization**
- Hostnames and SANs should be structured lists.

**Thread safety**
- Safe to emit from concurrent connection tasks because certificate generation may be shared.

**Lifetime**
- Transient event, usually one per hostname cache entry.

**Storage policy**
- Keep only in session diagnostics unless the user asks for detailed TLS audit logs.

**Versioning**
- Additional certificate profile fields should be additive.

**Trade-off**
- Useful for debugging trust failures, but sensitive enough that it should stay terse by default.

### 5. `CertificateIssued`
**Purpose**
Record that a leaf certificate was successfully generated or retrieved.

**Fields**
- envelope fields
- `hostname`
- `issuer_fingerprint`
- `leaf_fingerprint`
- `valid_from`
- `valid_to`
- `cache_key`

**Ownership**
- Owned by the TLS manager.

**Serialization**
- Serialize validity times using a stable timestamp format.

**Thread safety**
- Safe for concurrent use in trust diagnostics.

**Lifetime**
- Short-lived but important for audits and troubleshooting.

**Storage policy**
- Store in session history and doctor output, not in long-lived raw payload archives.

**Versioning**
- Fingerprint format changes must be versioned carefully.

**Trade-off**
- This event helps explain why TLS interception succeeded, but it also increases the amount of security-sensitive metadata in memory.

### 6. `CertificateInstallRequested`
**Purpose**
Record that the user requested trust-store installation or removal.

**Fields**
- envelope fields
- `action` (`install`, `uninstall`, `export`, `path`)
- `trust_store_target`
- `platform`
- `requires_elevation`

**Ownership**
- Owned by the CLI and TLS management layer.

**Serialization**
- Serialize the requested action and platform explicitly.

**Thread safety**
- Safe to send to background task workers.

**Lifetime**
- One event per trust management command.

**Storage policy**
- Keep in command history and diagnostics, not in flow stores.

**Versioning**
- New trust-store actions should be added as new variants.

**Trade-off**
- This gives a complete trust audit trail, but it is separate from the live proxy flow model.

### 7. `TlsHandshakeStarted`
**Purpose**
Record the beginning of a TLS handshake on either side of the proxy.

**Fields**
- envelope fields
- `side` (`client`, `upstream`)
- `hostname`
- `alpn_offered`
- `sni`
- `handshake_profile`

**Ownership**
- Owned by the TLS transport layer.

**Serialization**
- Keep ALPN and SNI fields structured and optional.

**Thread safety**
- Safe to send through the connection task.

**Lifetime**
- Brief event emitted once per handshake attempt.

**Storage policy**
- Store alongside the session to explain handshake failures and protocol negotiation.

**Versioning**
- New handshake metadata should be additive.

**Trade-off**
- This is invaluable for debugging TLS issues, but it should remain compact to avoid turning every handshake into a verbose audit log.

### 8. `TlsHandshakeCompleted`
**Purpose**
Record that TLS negotiation succeeded.

**Fields**
- envelope fields
- `side`
- `protocol`
- `cipher_suite`
- `peer_cert_chain_fingerprint`
- `negotiated_alpn`
- `session_resumed`

**Ownership**
- Owned by the TLS layer until emitted.

**Serialization**
- Serialize negotiated protocol and cipher data in stable enum/string form.

**Thread safety**
- Safe to share across the session and observability tasks.

**Lifetime**
- Emitted once per successful TLS negotiation side.

**Storage policy**
- Store in the session history and TLS diagnostics.

**Versioning**
- Cipher and protocol catalogs will evolve, so readers must tolerate unknown variants.

**Trade-off**
- Rich handshake details help explain performance and compatibility, but they increase the sensitivity of stored metadata.

### 9. `TlsHandshakeFailed`
**Purpose**
Record that TLS negotiation failed.

**Fields**
- envelope fields
- `side`
- `error_code`
- `error_kind`
- `retryable`
- `peer_hostname`
- `stage`

**Ownership**
- Owned by the TLS layer.

**Serialization**
- Encode errors as stable kinds plus optional human text.

**Thread safety**
- Safe to move to diagnostics and doctor output.

**Lifetime**
- Emitted once per failed attempt.

**Storage policy**
- Store in session history and troubleshooting logs.

**Versioning**
- New failure kinds should be additive and unknown kinds should not crash readers.

**Trade-off**
- Clear failure classification improves supportability, but it also increases the surface area of error taxonomy.

### 10. `BytesRead`
**Purpose**
Record bytes read from a socket or stream.

**Fields**
- envelope fields
- `side`
- `byte_count`
- `buffer_id`
- `is_partial`
- `eof_seen`
- `truncated`

**Ownership**
- The buffer owner keeps the actual bytes; the event only references the byte count and storage handle.

**Serialization**
- Never serialize raw bytes inline unless explicitly requested by a capture format.

**Thread safety**
- Safe to emit from the I/O task into the decode path.

**Lifetime**
- Very short-lived as a pipeline event, longer-lived only if attached to a capture record.

**Storage policy**
- Keep as a compact counter event in hot memory; persist only in debug or replay modes.

**Versioning**
- Buffer identity fields are implementation-specific and should be versioned carefully or replaced with opaque IDs.

**Trade-off**
- This event is great for throughput and backpressure analysis, but raw read events can become noisy if overused.

### 11. `BytesWritten`
**Purpose**
Record bytes written to a socket or stream.

**Fields**
- envelope fields
- `side`
- `byte_count`
- `buffer_id`
- `is_partial`
- `eof_seen`
- `truncated`

**Ownership**
- Same ownership model as `BytesRead`.

**Serialization**
- Store counts, not raw bytes, unless a special capture format requests payload logging.

**Thread safety**
- Safe to emit from the forwarding path.

**Lifetime**
- Short-lived in the event pipeline.

**Storage policy**
- Keep only when needed for performance or replay diagnostics.

**Versioning**
- Must remain structurally consistent with `BytesRead`.

**Trade-off**
- Separating read and write events helps diagnose asymmetry, but it doubles the number of byte-level metrics.

### 12. `ProtocolDetected`
**Purpose**
Record the protocol guessed or confirmed for a flow.

**Fields**
- envelope fields
- `protocol`
- `confidence`
- `detection_source`
- `evidence`
- `detector_version`

**Ownership**
- Owned by the detection stage.

**Serialization**
- Confidence and evidence should be structured so later versions can refine the detector without breaking readers.

**Thread safety**
- Safe to share across store, UI, and export tasks.

**Lifetime**
- Emitted early and may be updated if a stronger signal arrives.

**Storage policy**
- Persist in the flow record and protocol index.

**Versioning**
- Additive evidence types should not force a new event type.

**Trade-off**
- Early detection improves UX, but it can be wrong until more bytes arrive.

### 13. `FrameDecoded`
**Purpose**
Record that a protocol frame or message boundary has been decoded.

**Fields**
- envelope fields
- `protocol`
- `frame_id`
- `frame_kind`
- `message_order`
- `header_summary`
- `body_handle`
- `body_length`
- `truncated`

**Ownership**
- The decoder owns the frame until it is emitted.
- After emission, the store owns the canonical frame record and any shared body handle.

**Serialization**
- Serialize headers as structured key-value pairs.
- Serialize the body as a handle or blob depending on the output format.

**Thread safety**
- Must be `Send + Sync` because it may cross from the decode worker to the store actor.

**Lifetime**
- Medium-lived in the store, short-lived in the decode pipeline.

**Storage policy**
- Keep frame metadata in memory.
- Keep body data capped, shared, and evictable.

**Versioning**
- Protocol-specific frame fields belong in nested, versioned payloads.

**Trade-off**
- Frame events are the most useful unit for inspection, but they are also the most structurally complex.

### 14. `RedactionApplied`
**Purpose**
Record that sensitive data was masked.

**Fields**
- envelope fields
- `rule_id`
- `redacted_fields`
- `redaction_mode`
- `severity`
- `was_reveal_allowed`

**Ownership**
- Owned by the redaction stage.

**Serialization**
- Preserve enough information to explain why a field was masked without restoring the secret itself.

**Thread safety**
- Safe to share with the UI and export layers.

**Lifetime**
- Emitted once per redaction operation or per redacted message, depending on the protocol.

**Storage policy**
- Keep in flow history and export metadata, but never keep unredacted secrets in this event.

**Versioning**
- Redaction rule schemas should be versioned independently of the event envelope.

**Trade-off**
- The event is essential for trust, but it must be designed so the explanation never leaks the thing it is hiding.

### 15. `MessageEmitted`
**Purpose**
Record that a fully normalized message is ready for storage, UI rendering, or export.

**Fields**
- envelope fields
- `message_id`
- `frame_id`
- `flow_id`
- `summary`
- `status_code`
- `latency_ms`
- `sensitivity`
- `body_handle`

**Ownership**
- Owned by the store after emission.

**Serialization**
- Use a stable schema with explicit message ordering and summary fields.

**Thread safety**
- Safe to move across thread boundaries and store in the bounded actor.

**Lifetime**
- Medium-lived in memory; long-lived only when exported.

**Storage policy**
- Store as the canonical inspectable unit of work.

**Versioning**
- Summary fields may be extended, but message identity and ordering must remain stable.

**Trade-off**
- This is the event most users think of as "the request," but it is intentionally separate from lower-level transport and decode events.

### 16. `FlowUpdated`
**Purpose**
Record that the aggregate flow state changed.

**Fields**
- envelope fields
- `flow_id`
- `message_count`
- `open_state`
- `status`
- `peer_summary`
- `rolling_latency`
- `dropped_event_count`

**Ownership**
- Owned by the store actor.

**Serialization**
- Serialize as a compact aggregate record.

**Thread safety**
- Safe to broadcast to the TUI and exporters.

**Lifetime**
- Medium-lived as a live summary and longer-lived in snapshots.

**Storage policy**
- Keep as the source for flow map rendering and filtering.

**Versioning**
- New aggregate metrics should be added carefully because the UI depends on field stability.

**Trade-off**
- Aggregate flow events keep the UI fast, but they intentionally hide per-message detail.

### 17. `MetricsUpdated`
**Purpose**
Record operational counters such as throughput, queue depth, drops, and memory pressure.

**Fields**
- envelope fields
- `queue_depth`
- `bytes_in`
- `bytes_out`
- `event_drops`
- `evictions`
- `active_sessions`
- `resident_bytes_estimate`

**Ownership**
- Owned by the metrics collector or store actor.

**Serialization**
- Serialize counters as numbers, not strings, to keep export and telemetry easy to consume.

**Thread safety**
- Safe for concurrent read access from the UI.

**Lifetime**
- Very short in live updates, longer in exported snapshots.

**Storage policy**
- Keep in ring-buffer summaries and diagnostic snapshots.

**Versioning**
- New counters should not break existing readers as long as the field set remains additive.

**Trade-off**
- Metrics are indispensable for understanding overload, but they can distract if shown too prominently.

### 18. `TimeoutExpired`
**Purpose**
Record that a phase-specific timeout was reached.

**Fields**
- envelope fields
- `timeout_kind`
- `elapsed_ms`
- `limit_ms`
- `session_id`
- `phase`

**Ownership**
- Owned by the session timeout manager.

**Serialization**
- Encode the timeout kind and phase as stable enums.

**Thread safety**
- Safe to emit from timer tasks into the session state machine.

**Lifetime**
- Brief event, but important for error diagnostics.

**Storage policy**
- Store in session history and doctor output.

**Versioning**
- Future timeout phases should be introduced as new enum variants.

**Trade-off**
- Timeouts prevent hangs, but they can also surface as false positives if defaults are too strict.

### 19. `SessionDrainingStarted`
**Purpose**
Record that shutdown has begun and the session is draining rather than accepting new work.

**Fields**
- envelope fields
- `reason`
- `grace_period_ms`
- `active_sessions`

**Ownership**
- Owned by shutdown coordination.

**Serialization**
- Serialize the shutdown reason and grace period explicitly.

**Thread safety**
- Safe to broadcast to all session tasks.

**Lifetime**
- Short-lived during shutdown.

**Storage policy**
- Keep in shutdown diagnostics, not in regular flow history unless the user exports a shutdown trace.

**Versioning**
- Additive reasons are fine as long as unknown values are tolerated.

**Trade-off**
- Draining gives graceful exit behavior, but it needs careful coordination to avoid delaying shutdown forever.

### 20. `SessionClosed`
**Purpose**
Record that the session ended normally.

**Fields**
- envelope fields
- `close_reason`
- `bytes_read`
- `bytes_written`
- `frames_decoded`
- `duration_ms`

**Ownership**
- Owned by the session finalizer.

**Serialization**
- Serialize the close reason and summary counters in stable numeric or enum form.

**Thread safety**
- Safe to emit after all connection tasks have settled.

**Lifetime**
- Final event in the session lifecycle.

**Storage policy**
- Store as the closing summary for the flow.

**Versioning**
- Summary counters may grow over time, but the close event must remain compact and stable.

**Trade-off**
- Final close summaries are very useful, but they must not duplicate so much detail that storage becomes wasteful.

### 21. `SessionFailed`
**Purpose**
Record that the session terminated because of an error.

**Fields**
- envelope fields
- `error_kind`
- `error_code`
- `phase`
- `recoverable`
- `action_hint`

**Ownership**
- Owned by the relevant failing subsystem, then consolidated by the finalizer.

**Serialization**
- Use stable error kinds plus human-readable hints.

**Thread safety**
- Safe to move into diagnostics and UI status panels.

**Lifetime**
- Final event for the failed session.

**Storage policy**
- Persist in session history and troubleshooting output.

**Versioning**
- Unknown error kinds should be treated as generic failures by older readers.

**Trade-off**
- Rich failure details improve support, but they must not be so verbose that they obscure the root cause.

### 22. `ArtifactExported`
**Purpose**
Record that a capture or flow was written to disk.

**Fields**
- envelope fields
- `artifact_path`
- `format`
- `record_count`
- `redacted_record_count`
- `size_bytes`

**Ownership**
- Owned by the export layer.

**Serialization**
- Serialize the path, format, and counts explicitly.

**Thread safety**
- Safe to send from export workers back to the CLI.

**Lifetime**
- Short-lived command event.

**Storage policy**
- Keep in command history, not in raw flow data.

**Versioning**
- New export formats should be additive.

**Trade-off**
- Export events are small, but they are important for auditability and user trust.

### 23. `ReplayRequested`
**Purpose**
Record that a replay operation was initiated.

**Fields**
- envelope fields
- `source_artifact`
- `flow_selector`
- `target_override`
- `dry_run`
- `repeat_count`

**Ownership**
- Owned by the CLI replay command.

**Serialization**
- Serialize the replay plan rather than only the final outcome.

**Thread safety**
- Safe to pass from the CLI into the replay engine.

**Lifetime**
- Short-lived command event.

**Storage policy**
- Keep in command history and replay logs.

**Versioning**
- Replay options should be additive so older automation can still inspect the request.

**Trade-off**
- Recording the replay request is helpful for reproducibility, but it introduces another command lifecycle to maintain.

## Cross-Cutting Storage Policy

### Hot path
- Keep event envelopes and small summaries in memory.
- Share large payload bodies by handle or reference-counted bytes.
- Drop observability events before dropping forwarded traffic.

### Cold path
- Persist only flow summaries, exported artifacts, replay artifacts, and diagnostics the user asked to keep.
- Never store unredacted payloads by default.

### Eviction
- Oldest flow summaries may be evicted when memory is full.
- Metrics events should report evictions and drops clearly.
- Shutdown should flush what can be safely flushed within the grace period.

### Trade-offs
- Bounded storage keeps the tool usable on laptops.
- Eviction means history is finite, which is a deliberate trade for reliability.
- The event model favors later replay and export over permanent always-on capture.

## Serialization Strategy
- Use a versioned envelope around every event.
- Prefer structured data over opaque strings.
- Support at least one compact machine-oriented encoding and one readable diagnostic encoding.
- Redacted payloads should be serialized distinctly from raw payloads.
- Unknown fields must be ignored by older readers whenever possible.

### Trade-offs
- Versioned serialization adds engineering cost.
- It is necessary if the event model is going to remain stable while protocols, transports, and UI needs evolve.

## Thread Safety Strategy
- Events are immutable after creation.
- Large buffers are shared by handle or immutable byte storage.
- No event should require a mutable borrow once it leaves its originating stage.
- Cross-thread consumers should receive read-only snapshots or cloned envelopes only.

### Trade-offs
- This is a very strong constraint, but it prevents a large class of race conditions.
- The cost is a small amount of indirection and reference counting.

## Versioning Strategy
- Version the envelope and the per-event payload separately.
- Add fields additively when possible.
- Introduce new event types when semantics change in a breaking way.
- Readers should degrade gracefully when they encounter unknown event types or enum variants.

### Trade-offs
- Strict versioning creates more schema work up front.
- The benefit is long-term compatibility between the proxy, store, exports, and any future plugin or automation surface.

## Summary
The event model intentionally separates transport, protocol, storage, and lifecycle concerns. That makes the system easier to evolve without turning every component into a special case.

The central trade-off is that a clean event model requires discipline: more schema, more versioning, and more explicit ownership. The payoff is a reliable internal contract that can survive protocol growth, UI changes, replay, export, and long-running sessions.