# Architecture Decision Records

This document contains the initial architecture decisions for Lens. Each record is intentionally concise so it can be referenced during implementation and reviewed independently.

## ADR Index
- ADR 001: Proxy-first observation boundary
- ADR 002: Explicit TLS interception with a local CA
- ADR 003: TUI-first interface
- ADR 004: Bounded in-memory store
- ADR 005: Single-writer store actor
- ADR 006: Streaming incremental decoder API
- ADR 007: WASM plugin sandbox
- ADR 008: Redaction by default
- ADR 009: Multi-crate workspace boundaries
- ADR 010: Constructor injection over service locator
- ADR 011: Layered configuration precedence
- ADR 012: Structured logging with correlation IDs
- ADR 013: Optional eBPF as a discovery enhancer
- ADR 014: Snapshot-based UI model
- ADR 015: Runtime only in the I/O layer

## ADR 001: Proxy-first observation boundary
### Context
The original kernel-observability idea is fragile across platforms, TLS stacks, and managed cloud services. The product needs to be useful even when the user is on macOS or Windows and even when the upstream service is remote.

### Decision
Lens will observe traffic at the application boundary using a local proxy/interceptor as the primary architecture. Kernel-level discovery is optional and secondary.

### Alternatives
- eBPF-first capture
- Packet capture only
- A tracing agent injected into every application

### Pros
- Works across operating systems
- Sees decrypted traffic when the user explicitly routes through it
- Avoids root and kernel compatibility requirements in the main path

### Cons
- Requires setup or explicit routing
- Loses the magic of completely transparent kernel capture
- Some certificate-pinned applications cannot be intercepted

### Future implications
The proxy boundary becomes the stable contract for future features such as replay, exporters, and deeper protocol inspection.

## ADR 002: Explicit TLS interception with a local CA
### Context
Most useful developer traffic is encrypted. Without a deliberate trust model, the tool would only see ciphertext and would not deliver the promised protocol insight.

### Decision
Use explicit TLS interception with a locally trusted CA rather than attempting to read TLS from kernel or language-runtime hooks.

### Alternatives
- uprobe TLS hooks
- Raw ciphertext inspection only
- No TLS support at all

### Pros
- Reliable across common language runtimes
- Makes remote APIs and databases inspectable
- Avoids fragile per-library hooking logic

### Cons
- Requires user trust and CA installation
- Adds security responsibility around certificate handling
- Can be blocked by certificate pinning

### Future implications
The trust-store flow must remain easy to install, uninstall, and audit because it is part of the product's safety story.

## ADR 003: TUI-first interface
### Context
The product needs a clear default UI that works everywhere, including SSH sessions and local terminals, without adding browser deployment complexity.

### Decision
Ship the terminal UI as the primary interface and defer any web UI to a later phase.

### Alternatives
- Web-first dashboard
- Dual TUI and web UI MVP
- CLI-only output

### Pros
- Single-binary distribution stays simpler
- Works over SSH and in restricted environments
- Reduces browser, auth, and hosting concerns

### Cons
- Less visually expansive than a browser UI
- Some users prefer point-and-click workflows
- Complex visualizations are harder in a terminal

### Future implications
If a web surface is added later, it should consume the same store snapshots as the TUI rather than duplicating state logic.

## ADR 004: Bounded in-memory store
### Context
Traffic can be continuous and potentially high-volume. Unbounded retention would eventually exhaust memory on a developer laptop.

### Decision
Use a bounded in-memory store with explicit eviction and truncation policies.

### Alternatives
- Unbounded in-memory history
- Always-write-to-disk retention
- External database for capture storage

### Pros
- Predictable memory use
- Keeps the interactive experience responsive
- Makes resource caps explicit to the user

### Cons
- Older flows are eventually evicted
- Some long sessions lose detail
- Export is needed for long-term retention

### Future implications
The store should keep eviction counters and export hooks so the user can understand what was dropped and recover what matters.

## ADR 005: Single-writer store actor
### Context
Multiple connections can emit events concurrently, but shared mutable state with fine-grained locks would make correctness harder to reason about.

### Decision
Serialize store mutations through a single store actor that receives bounded messages.

### Alternatives
- Shared mutable state with locks
- Lock-free distributed indexes
- Partitioned stores per protocol or endpoint

### Pros
- Simplifies correctness and testing
- Avoids most lock contention
- Makes backpressure explicit through bounded queues

### Cons
- The actor can become a throughput bottleneck
- Requires careful message sizing
- Adds an internal queueing layer

### Future implications
If scaling becomes necessary, the actor can be sharded by flow family or protocol while preserving the same public store contract.

## ADR 006: Streaming incremental decoder API
### Context
Traffic arrives in fragments, may be pipelined, and can be large. A decoder that expects complete messages would fail on real networks.

### Decision
Define protocol decoders as streaming incremental state machines with explicit partial-read and desync handling.

### Alternatives
- Whole-message parsing only
- Packet-level heuristics
- One decoder per request/response pair with blocking reassembly

### Pros
- Works with fragmented and interleaved traffic
- Handles large payloads without unbounded buffering
- Creates a reusable protocol contract for built-ins and plugins

### Cons
- Harder to implement and test
- Requires more state management per flow
- The API is stricter than a batch parser

### Future implications
This contract should remain stable because plugin authors and built-in decoders will depend on it for compatibility.

## ADR 007: WASM plugin sandbox
### Context
Extensions may process sensitive payloads and should not receive arbitrary host privileges.

### Decision
Host plugins as WASM components with explicit installation, versioned ABI, and resource limits.

### Alternatives
- Native dynamic libraries
- Script plugins with host process access
- No plugin system

### Pros
- Strong isolation boundary
- Portable across operating systems
- Easier to reason about than ABI-stable native loading

### Cons
- More runtime overhead
- More packaging complexity
- Harder to debug than direct in-process code

### Future implications
The plugin ABI will need compatibility rules and migration support, but the sandbox gives room to evolve without risking the host process.

## ADR 008: Redaction by default
### Context
The tool intentionally sees plaintext secrets once TLS is decrypted. Unsafe defaults would make screenshots and exports risky.

### Decision
Redaction is enabled by default, and any reveal mode must be an explicit opt-in.

### Alternatives
- Reveal by default
- User-configurable masking only
- No redaction and rely on user caution

### Pros
- Safer by default
- Makes demos and screenshots less risky
- Encourages disciplined sharing of captured data

### Cons
- Can hide information a user wants during debugging
- Requires careful rule design to avoid false positives and negatives
- Adds another transformation stage to the pipeline

### Future implications
Any export, replay, or plugin surface that can expose bodies must respect the same redaction policy by default.

## ADR 009: Multi-crate workspace boundaries
### Context
The product spans networking, platform integration, protocol parsing, storage, UI, and extension hosting. A monolith would make platform-specific churn leak everywhere.

### Decision
Organize the project as a multi-crate workspace with narrow responsibilities per crate.

### Alternatives
- One large crate
- A handful of broad packages
- A monorepo without explicit module boundaries

### Pros
- Strong dependency boundaries
- Better incremental compilation and test isolation
- Easier to keep optional features out of the core build

### Cons
- More build and release complexity
- More cross-crate versioning discipline
- Higher initial scaffolding overhead

### Future implications
The workspace can grow new adapters and plugins without forcing the core model to absorb every new concern.

## ADR 010: Constructor injection over service locator
### Context
The system needs to be testable, portable, and explicit about its dependencies. Hidden globals would make the architecture hard to verify.

### Decision
Use constructor injection and keep the CLI as the composition root.

### Alternatives
- Global singletons
- Service locator registry
- Static mutable configuration

### Pros
- Clear dependency graphs
- Easier unit tests and mocks
- Fewer hidden coupling points

### Cons
- More wiring code
- Some constructors become long
- Requires discipline to keep dependencies small

### Future implications
As the system grows, constructor-based wiring makes it easier to swap platform adapters, stores, and decoders without rewriting call sites.

## ADR 011: Layered configuration precedence
### Context
Users will want both quick one-off runs and repeatable project settings. Different layers of configuration are useful in different contexts.

### Decision
Use the precedence order: flags, environment variables, project config, user config, defaults.

### Alternatives
- Flags only
- A single config file
- Environment variables only

### Pros
- Supports both ad hoc and repeatable use
- Gives users predictable override behavior
- Works well in CI, local shells, and shared project setups

### Cons
- Harder to reason about than one source of truth
- Requires a resolved-config diagnostic command
- Risk of confusion if precedence is undocumented

### Future implications
New settings must fit into the same precedence model so the system remains predictable.

## ADR 012: Structured logging with correlation IDs
### Context
Debugging a multi-connection, multi-module system is difficult without strong log context, but logs may contain sensitive payload metadata.

### Decision
Emit structured logs with correlation IDs, protocol labels, and sensitivity markers, and keep them redacted by default.

### Alternatives
- Free-form text logs
- Very verbose debug dumps
- No application logs beyond the UI

### Pros
- Easier machine parsing and filtering
- Better correlation across proxy, decoder, store, and UI layers
- Safer to share than raw payload logs

### Cons
- Requires logging discipline everywhere
- Can hide detail that some debugging sessions need
- Structured logs are more work than print statements

### Future implications
Observability tools, exporters, and diagnostics should consume the same structured schema instead of inventing parallel formats.

## ADR 013: Optional eBPF as a discovery enhancer
### Context
Kernel-level discovery can be magical on Linux, but it must not be required for the product to work.

### Decision
Treat eBPF as an optional discovery enhancement that can feed connection metadata into the store when available.

### Alternatives
- Mandatory eBPF core architecture
- No eBPF support
- eBPF for every platform via shims

### Pros
- Retains the possibility of zero-configuration discovery on Linux
- Does not block cross-platform usability
- Keeps kernel-specific risk out of the primary path

### Cons
- Linux-only feature for the enhancement path
- Additional maintenance burden if enabled
- Secondary path can never be relied on as the only source of truth

### Future implications
The optional path should remain additive and feature-gated so the main product always works without it.

## ADR 014: Snapshot-based UI model
### Context
The UI should remain responsive while traffic continues to flow, and it should never mutate the store directly.

### Decision
Render the UI from immutable store snapshots rather than live shared state.

### Alternatives
- Direct UI mutation of shared state
- Live subscription to every store event
- Polling individual mutable fields

### Pros
- Safer and easier to reason about
- Decouples rendering cadence from event ingestion
- Makes testing the UI more deterministic

### Cons
- UI can lag behind the freshest event by one snapshot interval
- Snapshot generation has some overhead
- Large snapshots can increase memory churn if not bounded

### Future implications
Any future web UI or export pipeline should use the same snapshot model to stay consistent with the TUI.

## ADR 015: Runtime only in the I/O layer
### Context
The project needs asynchronous networking, but the core domain model should not depend on a specific async framework.

### Decision
Confine the async runtime to I/O, timers, and orchestration layers. Keep domain logic runtime-agnostic and push blocking work off the hot path.

### Alternatives
- Runtime across the entire codebase
- Synchronous networking only
- Multiple nested runtimes

### Pros
- Keeps the core portable and easier to test
- Simplifies cancellation and backpressure handling
- Avoids leaking framework details into business logic

### Cons
- Requires more boundary code
- Adds a runtime dependency to the application shell
- Some operations need explicit task management

### Future implications
If the runtime or async ecosystem changes, the core domain layer should remain largely unaffected because the runtime is isolated at the edge.

## Closing note
These decisions intentionally bias toward portability, bounded resource usage, explicit safety controls, and a stable extension surface. They trade away some of the original kernel-level magic in exchange for a system that is more reliable, easier to ship, and easier to evolve.