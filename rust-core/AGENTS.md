# rust-core Workspace

## OVERVIEW

Rust workspace (resolver="2") with 10 crates. `audio-core` and `audio-codec` are the only real implementations; the other 8 are skeletons.

## STRUCTURE

```
crates/
├── audio-core/      # FOUNDATION: Sample trait, AudioBuffer<T>, AudioFormat, AudioError
├── audio-codec/     # COMPLETE: Opus encode/decode, 22 tests, Criterion benchmarks
├── audio-capture/   # skeleton
├── audio-playback/  # skeleton
├── audio-processor/ # skeleton (AEC/NS/AGC)
├── audio-mixer/     # skeleton
├── network/         # skeleton (UDP/QUIC)
├── discovery/       # skeleton (mDNS)
├── protocol/        # skeleton (serialization)
└── ffi-bindings/    # skeleton (C/JNI)
```

## WHERE TO LOOK

| Task | Crate | Notes |
|------|-------|-------|
| Audio types & errors | `audio-core` | `Sample`, `AudioBuffer<T>`, `AudioFormat`, `AudioError` |
| Opus codec work | `audio-codec` | Reference implementation, read its `AI_GUIDE.md` first |
| Codec benchmarks | `audio-codec/benches/` | Criterion 0.5, `cargo bench -p audio-codec` |
| Skeleton crate impl | any skeleton crate | Each has `AI_GUIDE.md` with next steps |
| FFI/C interop | `ffi-bindings` | Will bridge Rust to C for Windows/Android JNI |

## WORKSPACE DEPENDENCIES

Defined in root `Cargo.toml` `[workspace.dependencies]`. Key versions:

- `opus = "0.3.0"` (audio-codec only)
- `cpal = "0.15.2"` (capture/playback, not yet used)
- `tokio = "1.35"` full features
- `quinn = "0.10"` (network, not yet used)
- `thiserror = "1.0"` (error derive, all crates)
- `tracing = "0.1"` / `tracing-subscriber = "0.3"`
- `criterion = "0.5"` (dev-dependency, audio-codec only)
- `serde = "1.0"` with derive
- `bytes = "1.5"`, `crossbeam-channel = "0.5"`, `parking_lot = "0.12"`, `dashmap = "5.5"`

Crates use `foo.workspace = true` to inherit. Don't pin versions in individual `Cargo.toml` files.

## CONVENTIONS

- **Error handling**: `audio-core` defines `AudioError`. Skeleton crates reference `audio_core::Result` (type alias to be added). `audio-codec` defines its own `CodecError` with `thiserror`.
- **Skeleton pattern**: `_private: ()` field, `new() -> Result<Self>` factory, `unimplemented!()` in methods.
- **Test naming**: `{name}_test.rs` in `tests/` dir (project convention, not Rust standard).
- **Benchmarking**: Only `audio-codec` has Criterion benchmarks (`benches/opus_benchmark.rs`).
- **audio-core API stability**: Changes here break all other crates. Treat as public API.
- **`audio-codec` is the reference**: When implementing a new crate, study its structure, error pattern, and test coverage first.

## ANTI-PATTERNS

- Don't add deps to individual crates without checking `[workspace.dependencies]` first
- Don't use `unwrap()` in library code, propagate with `Result`
- Don't bypass the `_private: ()` pattern in skeleton crates (prevents external construction)
- Don't modify `audio-core` types without considering downstream breakage
- Don't create tests as `{name}_test.rs` if you're adding a new convention, match existing style
