# rust-core Workspace

## OVERVIEW

Rust workspace (resolver="2") with 10 crates. All crates are fully implemented with 405 tests passing.

## STRUCTURE

```
crates/
├── audio-core/      # COMPLETE: Sample trait, AudioBuffer<T>, AudioFormat, AudioError
├── audio-codec/     # COMPLETE: Opus encode/decode, 22 tests, Criterion benchmarks
├── audio-capture/   # COMPLETE: cpal implementation, Fixed(960) frames
├── audio-playback/  # COMPLETE: cpal implementation, mixer output
├── audio-processor/ # COMPLETE: AEC/NS/AGC (NLMS/SNR/Attack-Release)
├── audio-mixer/     # COMPLETE: Mix engine, soft_clip tanh
├── network/         # COMPLETE: UDP transport, zero-copy serialization
├── discovery/       # COMPLETE: mDNS (mdns_sd implementation)
├── protocol/        # COMPLETE: 12-byte header, zero-copy protocol
└── ffi-bindings/    # COMPLETE: Full pipeline (capture→encode→send + recv→decode→play)
```

## WHERE TO LOOK

| Task | Crate | Notes |
|------|-------|-------|
| Audio types & errors | `audio-core` | `Sample`, `AudioBuffer<T>`, `AudioFormat`, `AudioError` |
| Opus codec work | `audio-codec` | Reference implementation, read its `AI_GUIDE.md` first |
| Codec benchmarks | `audio-codec/benches/` | Criterion 0.5, `cargo bench -p audio-codec` |
| FFI bindings | `ffi-bindings` | Full pipeline (capture→encode→send + recv→decode→play), 76 tests |
| Audio capture | `audio-capture` | cpal implementation, Fixed(960) frames |
| Audio playback | `audio-playback` | cpal implementation, mixer output |
| Audio processing | `audio-processor` | AEC/NS/AGC (NLMS/SNR/Attack-Release) |
| Audio mixing | `audio-mixer` | Mix engine, soft_clip tanh |
| Network transport | `network` | UDP transport, zero-copy serialization |
| Device discovery | `discovery` | mDNS (mdns_sd implementation) |
| Protocol | `protocol` | 12-byte header, zero-copy protocol |

## WORKSPACE DEPENDENCIES

Defined in root `Cargo.toml` `[workspace.dependencies]`. Key versions:

- `opus = "0.3.0"` (audio-codec only)
- `cpal = "0.15.2"` (capture/playback)
- `tokio = "1.35"` full features
- `thiserror = "1.0"` (error derive, all crates)
- `tracing = "0.1"` / `tracing-subscriber = "0.3"`
- `criterion = "0.5"` (dev-dependency, audio-codec only)
- `serde = "1.0"` with derive
- `bytes = "1.5"`, `crossbeam-channel = "0.5"`, `parking_lot = "0.12"`, `dashmap = "5.5"`

Crates use `foo.workspace = true` to inherit. Don't pin versions in individual `Cargo.toml` files.

## CONVENTIONS

- **Error handling**: `audio-core` defines `AudioError`. All crates reference `audio_core::Result`. `audio-codec` defines its own `CodecError` with `thiserror`.
- **Factory pattern**: `_private: ()` field, `new() -> Result<Self>` factory, prevents external construction.
- **Test naming**: `{name}_test.rs` in `tests/` dir (project convention, not Rust standard).
- **Benchmarking**: Only `audio-codec` has Criterion benchmarks (`benches/opus_benchmark.rs`).
- **audio-core API stability**: Changes here break all other crates. Treat as public API.
- **`audio-codec` is the reference**: When implementing a new crate, study its structure, error pattern, and test coverage first.

## ANTI-PATTERNS

- Don't add deps to individual crates without checking `[workspace.dependencies]` first
- Don't use `unwrap()` in library code, propagate with `Result`
- Don't bypass the `_private: ()` pattern in crates (prevents external construction)
- Don't modify `audio-core` types without considering downstream breakage
- Don't create tests as `{name}_test.rs` if you're adding a new convention, match existing style
