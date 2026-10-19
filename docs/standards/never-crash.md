# Never crash

People trust Protasena with their designs; a crash loses their work. This outranks feature work.

1. **No panics in non-test code.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`,
   `unimplemented!`, and no indexing or slicing that can panic. The workspace
   `[workspace.lints.clippy]` in `Cargo.toml` denies `unwrap_used`, `expect_used`, `panic`,
   `indexing_slicing`, `todo`, `unimplemented` and `unreachable`; `clippy.toml` allows the first
   four inside tests. Every crate has `[lints] workspace = true`.
2. **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`.
3. **Errors are values.** `Result<T, E>` with the crate's error type and `?`. An unfinished
   feature returns an error or says "not available yet".
4. **Input is hostile.** KiCad files, library packs, project files, `.pdsprj` containers,
   command parameters and control-channel messages: `get()` not `[i]`, checked arithmetic, no
   NaN-to-int casts, caps on input-sized allocations and loops, bounded recursion (depth limits
   in every parser).
5. **Last-resort guard.** Command execution in the engine runs under `catch_unwind`; an escaped
   panic restores the design and reports an error.
6. **Prove it.** Every crash fix lands with a regression test (pattern:
   `hostile_<input>_never_panics`).
