# AGENTS.md — HISTORY

Recorded decisions with git references. Read when relevant to current task.
Acts as simple long-term memory for the project.

## Format

| Date       | Decision   | Rationale   | Git ref          |
| ---------- | ---------- | ----------- | ---------------- |
| YYYY-MM-DD | [describe] | [why]       | [commit hash/tag] |

## Guidance

- Record decisions that would be costly to rediscover.
- Note false turns and why they were rejected.
- Link to relevant commits.
- Keep entries brief — enough to reconstruct reasoning.
- When this file grows too large, archive older entries to
  `.agents/history/` and leave a pointer here.

## Entries

| Date       | Decision   | Rationale   | Git ref          |
| ---------- | ---------- | ----------- | ---------------- |
| 2026-09-15 | OLA fast path for 1:1 FIR (dwn_up == 1, h0.len() >= 256): rustfft overlap-add with chunk-size ~4096, fft_len = next_pow2(4096 + 2*(taps-1)). Reuses FFT plan per filter. y_out[j] = y_full[history_len + j] (no +overlap). BlockwiseFilter state format unchanged. Rate-change kernels untouched. | bsqp-2 perf brief: bp20k_48khz 4s mono 563 ms -> <=30 ms. Achieved 4.0 ms (144x). | see commit |
| 2026-09-15 | N-block == 1-block and snapshot/restore for OLA filters use `allclose(atol=1e-5)`, not bit-exact `==`. The OLA f64 add-order noise is ~1e-13 per output and accumulates to ~1e-6 across thousands of FFT round-trips. The G.191 ±1 int16 margin is 1.5e-4; 1e-5 sits ~15x below. | per bsqp-2 clarification; the existing matched-rate `assert_eq!` in lib.rs:902 still holds because rate-matched 1:1 filtering routes through the direct path. | n/a |
