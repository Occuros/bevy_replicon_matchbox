# Bevy Replicon Matchbox

This crate integrates [`matchbox`](https://github.com/johanhelsing/matchbox) as a backend for [`bevy_replicon`](https://github.com/komora-io/bevy_replicon), enabling peer-to-peer networking via WebRTC, STUN, and ICE.

Matchbox provides convenient NAT traversal support out of the box — no need to manually manage signaling, host discovery, or ICE negotiation.

> ⚠️ **Note**: This is an early implementation and may still contain bugs or limitations.

---

## Running an Example

To run one of the examples from the [`examples`](examples) directory:

```bash
cargo run --example <example_name> server
```

in another terminal
```bash
cargo run --example <example_name> client
```

Each example starts a host peer that also acts as the listen server.

For production setups, it’s recommended to use a dedicated matchbox signaling server.



### Known Limitations

- **Empty message workaround**  
  WebRTC can silently drop empty messages. To prevent this, each message is currently prefixed with a single `byte` to ensure delivery.


- **WASM support not verified (yet)**  
  This backend has not been tested in WebAssembly environments. Compatibility is currently unverified.

## Compatible versions

| bevy | bevy_matchbox | bevy_replicon | bevy_replicon_renet |
|------|---------------|---------------|---------------------|
| 0.16 | 0.12          | 0.34          | 0.16                |


## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.