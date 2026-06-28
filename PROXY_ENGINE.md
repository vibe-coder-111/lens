# Proxy Engine Architecture

## Scope
This document designs the proxy engine only. It covers connection lifecycle, TLS interception, certificate management, HTTP processing, buffers, streaming, backpressure, timeouts, shutdown, and memory limits.

The proxy engine is the product's data plane. Its job is to forward traffic correctly and quickly while making inspection possible without letting observability slow down the user's application.

## Design Goals
- Forward traffic reliably under normal and degraded conditions.
- Keep the hot path independent from the UI and most storage work.
- Support explicit proxy and transparent interception modes.
- Terminate and originate TLS locally when the user has opted into inspection.
- Preserve enough payload and metadata for debugging without allowing memory growth to become unbounded.
- Fail closed for trust and certificate problems, but fail open for observability whenever the user's traffic would otherwise be interrupted.

### Trade-offs
- A proxy-first design is more portable than kernel capture, but it requires explicit routing or certificate trust setup.
- Separating forwarding from observability improves reliability, but it means some diagnostic data may be dropped under extreme load.
- Strict safety defaults make the proxy more trustworthy, but they also add setup steps for first-time users.

## High-Level Flow
1. Accept a connection.
2. Classify it as explicit proxy traffic or intercepted traffic.
3. Resolve the upstream destination.
4. Decide whether TLS is end-to-end, intercepted, or plain text.
5. Create a per-connection session object.
6. Start bidirectional forwarding.
7. Mirror bytes into the decode and store pipeline.
8. Apply protocol detection, HTTP parsing, and redaction.
9. Emit flow and message events to the store.
10. Close the session when both directions finish or a timeout/shutdown occurs.

### Trade-offs
- A single session object makes the lifecycle easy to reason about, but it requires careful state ownership.
- Early classification simplifies downstream logic, but it means the accept path must peek enough metadata to decide the mode quickly.

## Connection Lifecycle
### Decision
Every inbound socket becomes a session with an explicit state machine:
- `Accepted`
- `Classified`
- `ResolvingUpstream`
- `TLSSetup`
- `Forwarding`
- `Draining`
- `Closed`
- `Failed`

Each session owns:
- the client-side stream
- the upstream-side stream
- protocol detection state
- buffer cursors
- timeout bookkeeping
- metrics and counters

### Trade-offs
- An explicit state machine is more verbose than implicit async callbacks, but it makes shutdown, cancellation, and error handling predictable.
- Session-local state prevents accidental cross-connection contamination, but it increases per-connection overhead slightly.
- Separate states for draining and closed add complexity, but they avoid truncating in-flight responses during shutdown.

## TLS Interception
### Decision
The proxy supports three TLS modes:
- plain pass-through for non-encrypted traffic
- explicit local termination for trusted interception
- tunnel mode for traffic that should not be inspected

When interception is enabled, the proxy presents a locally generated leaf certificate to the client and opens a separate TLS session to the upstream server.

### Trade-offs
- TLS termination gives visibility into decrypted payloads, which is the whole point of the product.
- TLS interception introduces trust-store setup and certificate handling risk.
- Supporting tunnel mode preserves compatibility for users who do not want inspection, but it reduces product value for that session.

## Certificate Management
### Decision
The proxy owns a local root CA and issues short-lived leaf certificates on demand. Leaf certs are cached by hostname and regenerated when expired. The CA is installed into the OS trust store only when the user explicitly asks for it.

Certificate handling rules:
- the CA private key stays local and user-scoped
- leaf certs are ephemeral and in-memory where possible
- certificate install and uninstall are reversible operations
- pinned or untrusted targets should produce clear diagnostics, not silent failure

### Trade-offs
- A local CA is operationally powerful, but it also becomes a sensitive secret that must be protected carefully.
- Caching leaf certs reduces handshake overhead, but it increases the amount of state the proxy must manage and expire correctly.
- Requiring explicit trust installation is safer, but it adds friction compared with silent interception.

## HTTP Pipeline
### Decision
The HTTP pipeline is a streaming sequence:
- byte capture
- protocol detection
- request/response framing
- header normalization
- body capture with limits
- redaction
- flow/message emission

The pipeline should treat HTTP/1.1 as the baseline, with future protocol layers able to plug into the same session model.

### Trade-offs
- A normalized HTTP pipeline makes the UI and export formats consistent.
- Parsing and normalizing headers costs CPU, but it pays off in better inspection and replay.
- A streaming pipeline is harder than a whole-request parser, but it avoids buffering entire bodies before any analysis begins.

## Buffer Management
### Decision
The proxy uses bounded buffers at every stage:
- per-direction read buffers for socket I/O
- per-session reassembly buffers for partial protocol records
- bounded message-body buffers for inspection and export
- bounded queues between proxy tasks and the store

Buffers are reused whenever possible to reduce allocation churn.

### Trade-offs
- Buffer reuse improves performance and reduces pressure on the allocator.
- Bounded buffers prevent memory blowups, but they can truncate payloads.
- Reassembly buffers make the stream parser simpler, but they can temporarily hold partial requests or responses.

## Streaming Model
### Decision
All decoding is incremental. The proxy never waits for a complete request before forwarding bytes. It accumulates enough state to identify a message boundary, then emits events as soon as data is available.

The streaming model distinguishes between:
- bytes needed to continue parsing
- bytes safe to forward immediately
- bytes selected for capture or redaction

### Trade-offs
- Incremental parsing handles fragmentation and large payloads well.
- It is more complex than batch parsing because each protocol must maintain parser state.
- The proxy stays responsive because forwarding does not depend on fully decoding the stream.

## Backpressure
### Decision
Backpressure is handled in layers:
- network forwarding uses natural socket backpressure
- observability events move through bounded channels
- if the observability path falls behind, it drops events rather than slowing the data plane
- if the data plane itself cannot keep up, the proxy favors correctness of forwarding over inspection completeness

### Trade-offs
- Dropping observability events preserves traffic latency, which is the right bias for a debugging tool.
- Backpressure-aware forwarding prevents runaway memory use, but it can surface as slower capture under load.
- Separating traffic reliability from diagnostic completeness is the safest compromise, even though it means some traces will be incomplete.

## Timeouts
### Decision
Timeouts are scoped by session and phase:
- accept/classification timeout
- upstream connect timeout
- TLS handshake timeout
- idle read timeout
- body capture timeout
- shutdown grace timeout

Timeouts should be configurable with conservative defaults and should emit clear diagnostics about the phase that expired.

### Trade-offs
- Phase-specific timeouts make failure causes much easier to understand.
- More timeout knobs mean more policy surface area.
- Conservative defaults reduce hangs, but they must not be so aggressive that slow but valid upstreams are broken.

## Shutdown
### Decision
Shutdown is coordinated in two stages:
1. stop accepting new sessions
2. drain in-flight sessions for a bounded grace period
3. cancel anything still active after the grace period

The proxy must preserve already-forwarded bytes and must attempt graceful closure on both client and upstream sockets.

### Trade-offs
- Graceful draining improves correctness for in-flight requests and responses.
- A hard stop is simpler, but it risks truncating responses and corrupting the user's session.
- Keeping a bounded grace period prevents shutdown from hanging indefinitely.

## Memory Limits
### Decision
Memory limits are enforced as first-class policy, not as a last-minute cleanup step. The proxy should cap:
- concurrent sessions
- per-session body size
- total buffered bytes
- queue depth between forwarding and observability
- cached certificate count

When limits are exceeded, the proxy should evict old diagnostic state or drop new observability data rather than let resident memory grow without bound.

### Trade-offs
- Hard limits keep the tool safe to leave running.
- Eviction can remove historical context that a user may want later.
- A bounded design is necessary on laptops and CI runners, where the proxy competes with the user's own workload for resources.

## Error Handling
### Decision
The proxy distinguishes between three error classes:
- connection errors, which affect one session
- trust/certificate errors, which may block inspection mode
- systemic errors, which affect the whole proxy process

Connection errors should fail the session cleanly. Trust errors should explain how to fix the environment. Systemic errors should stop the proxy only when continuing would risk incorrect forwarding.

### Trade-offs
- Fine-grained error classes improve diagnostics.
- More classes make the implementation a little more complex.
- The proxy should prefer explicit failure over ambiguous partial success when trust or TLS setup is wrong.

## Ownership Model
### Decision
The session owns the live connection state. The store owns canonical flow records. Diagnostic views receive immutable snapshots or reference-counted bodies, never mutable connection internals.

### Trade-offs
- Clear ownership avoids cross-thread aliasing bugs.
- Copying or sharing immutable snapshots introduces some overhead, but it keeps the proxy and UI decoupled.
- This model makes it easy to tear down a session without dangling references in the observer side.

## Module Shape
A sensible internal split for the proxy engine is:
- accept and classification
- upstream resolution
- TLS setup and certificates
- transport pump
- protocol framing and capture
- limits and policy enforcement
- lifecycle coordination
- diagnostics and metrics

### Trade-offs
- Splitting the engine into smaller responsibilities makes the code easier to test.
- Too many tiny modules can make control flow harder to follow, so boundaries should be functional rather than abstract for their own sake.

## Observability Contract
### Decision
The proxy emits structured events for:
- session start and end
- protocol detection
- TLS mode selection
- byte counts per direction
- body truncation
- timeout expiration
- event drops and eviction counts
- certificate and trust-store actions

### Trade-offs
- Rich events make debugging and benchmarks possible.
- Emitting too much low-level detail can create noise, so the event schema should be stable and intentionally small.

## Summary
The proxy engine should behave like a reliable traffic switch with a careful observability tap, not like a clever parser glued directly to sockets. Forwarding stays sacred, inspection is best-effort, and memory is always bounded. That gives the product a trustworthy core that can run continuously without becoming the thing it is trying to debug.