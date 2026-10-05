# FFT overlap-add acceleration of FirFilter (1:1 path)

## Goal
Make `filter_array` ≥ 20× faster on long-kernel 1:1 FIRs (bp20k_48khz 4001 taps,
p341_16khz 592 taps) while keeping ITU-T G.191 conformance, preserving the
public API (filter_array, BlockwiseFilter, get_state/set_state), and not
regressing small kernels or the rate-change path.

## Conformance margins (pre-change, cited)
- `tests/test_reference_verification.py:62` — `np.testing.assert_allclose(rust_i, stl_i, atol=1)` for IRS8/IRS16/HQ2/HQ3. Pre-change: 4/4 pass.
- `scripts/verify_stl.py:138` — `compare_signals(ref, ours, tolerance=1e-2)` for IRS8/IRS16/IRS48/HQ2/HQ3/FLAT. Pre-change: pass.
- Tolerance target for OLA output vs. the current direct path: `max |Δ| ≤ 1e-9` (f64 equality, the FIR impulse response is computed once; FFT and direct give the same polynomial product up to f64 rounding).

## Design

### Dispatch
- dwn_up > 1 (rate-change FIRs) — untouched. STL's transition + k0 logic is rate-specific; the user's brief explicitly excludes it.
- dwn_up == 1 AND h0.len() < `OLA_THRESHOLD` — direct path, unchanged. Captures rx_irs8 (148), flat1 (168), flat1to2 (168), msin16k (185), psophometric (156), dsm16k (207), tia_irs8 (148), rx_irs8 (148), rx_irs16 (148), hirs16 (168), lp1p5_48k (235), irs8 (256 — borderline; below threshold and stays direct).
- dwn_up == 1 AND h0.len() >= OLA_THRESHOLD — new OLA path. Captures p341_16khz (592), bp5k_16khz, bp100_5k_16khz, lp35_48k, lp7_48k, lp10_48k, lp12_48k, lp14_48k, lp20_48k, bp14k_32k, bp20k_48k (4001), irs16 (1040), mod_irs16 (1040), mod_irs48 (1040), bp100_5k_16khz.

### Threshold
- Threshold = 256. Justification: rx_irs8 (148 taps) at 1.79 ms/(s·ch) on 4s, lp1p5_48k (235 taps) at ~3 ms/(s·ch); OLA overhead at 256-tap FFT (size 512) is non-trivial vs. ~3 ms/(s·ch) direct. The real wins are at 4001 taps (141 ms/(s·ch)) — confirmed by scipy OLA's 7.5 ms in the brief.
- The threshold will be measured empirically (sweep 64/128/256/512/1024) once OLA is wired in; the chosen number recorded with the sweep data.

### OLA algorithm (1:1 FIR)
- Precomputed: H = FFT(h0 zero-padded to L=next_pow2(taps + max_block), L), so the same FFT plan is used for every input chunk.
- Per chunk: x_padded = [history (lenh0-1 samples) ++ chunk ++ zeros to L]; X = FFT(x_padded, L); Y = IFFT(X ⊙ H, L); y_out_chunk = Y[0..chunk.len()] + overlap_carryover (the previous chunk's Y[chunk.len()..chunk.len()+lenh0-1]).
- New state after chunk: history = chunk[chunk.len() - (lenh0 - 1) ..]; overlap_carryover = Y[chunk.len()..chunk.len() + (lenh0 - 1)].
- Edge case: chunk shorter than lenh0-1 — extend history by the chunk and emit a partial chunk result using last lenh0-1 of (history ++ chunk) convolved with the kernel (or just call direct path for short chunks).
- FFT size policy: `L = next_pow2(lenh0 + max_block)` where `max_block` = a sane chunk size for the OLA pipeline (e.g., 4096 or block_size); reusing one FFT plan per filter is the point.

### State serialization (compatibility with the public API)
- Today `FilterState::Fir { t, k0 }` flattens to `t ++ [k0]`. For 1:1 FIR `k0=0` so the user-visible state is just `t` of length `lenh0-1`.
- The OLA path has its own persistent state: `history` (last `lenh0-1` input samples) + `overlap_carry` (last `lenh0-1` output samples). To preserve the on-the-wire state format AND allow cross-path set_state, the decision is:
  - When the receiving filter is in OLA mode (dwn_up==1, taps >= threshold), `set_state` interprets the saved flat `t ++ [k0=0]` as the **input history** (`t`) and re-derives the **overlap_carry** by running the direct path on the input history (replaying through the t-only path) — this is O(lenh0), done once, not per chunk. Rationale: the input history uniquely determines what the OLA would have produced in its overlap region, so a one-shot replay is exact.
  - When the receiving filter is in direct mode and the saved state was OLA, the saved `t` field already contains the input history; we use it directly as the direct path's delay line.
  - `get_state` writes the **input history** (last `lenh0-1` input samples) as `t` with `k0=0`. This is the canonical state of a 1:1 FIR; the OLA's `overlap_carry` is derivable from it, so it is not separately serialized. Document this explicitly in the ADR-style comment near `get_state`/`set_state`.
- Net effect: existing serialized state from old releases is still valid AND the new path produces state that is still valid for the old path. The 1:1 1-block == N-block test (with and without snapshot) is the conformance test.

### Blockwise + set_state
- `BlockwiseFilter::process_block` / `process_chunk` already routes through `FirFilter::process_block`. The new OLA path lives in `FirFilter::process_block` itself. No public API change.
- `BlockwiseFilter` already slices input into `block_size` chunks. The OLA path's `max_block` is independent of `block_size` (it just affects the FFT size for one plan per filter); the per-chunk FFT is L-points each.

### Dependencies
- `rustfft` (MIT OR Apache-2.0). User task explicitly names "rustfft or equivalent" — sanctioned by the user, not the agent's choice. Add to `[dependencies]` of g191_filter_core.

### Test plan
- New Rust unit tests in `src/lib.rs::filter_samples_tests` (or a new module): equivalence of OLA vs. direct for many FIRs across many seeds and lengths; 1-block == N-block for the OLA path; snapshot/restore round-trip across the OLA path.
- New Python test in `tests/test_blockwise_state.py`: extend the existing state check (currently via temp script) to assert strict equality (1e-9) for all FIRs at the threshold boundary and a few above/below.
- New `tests/test_ola_perf.py` is NOT added (benchmarks live outside the test suite, per project convention). Bench harness is reused (in /tmp, not committed).

## Scope boundary
- `D:\SVN\Projects\SpeechQualityAssessment` is not touched. The consumer is told to bump their `g191-filter` floor.

## Risk
- Conformance margin vs STL: OLA == direct (same polynomial product), tolerance 1e-9 f64. The repo-defined G.191 margins (±1 int16, 1e-2 abs) are coarser — OLA is expected to pass them comfortably.
- Block-size interaction: OLA chunks internally at FFT-plan size, not at `block_size`. `process_chunk` slices input to `block_size` and calls `process_block` for each slice; the per-slice OLA round trip is the cost. No correctness change.

## Deliverables
1. Commit: implement OLA path.
2. Commit: threshold sweep + benches.
3. Commit: conformance numbers + state tests.
4. CalVer release so the consumer can raise their floor.
5. Intercom report with hashes, version, threshold rationale, bench numbers, conformance margins.

## Open question (will not block, will pick if needed)
- Should I also add a benchmark suite under `benches/` (criterion) for the OLA path? Not required by the brief, not blocking. Skip unless asked.
