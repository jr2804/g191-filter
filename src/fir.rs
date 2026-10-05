// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// FIR filter implementation matching STL reference algorithms
/// (fir-lib.c: fir_initialization, fir_downsampling_kernel, fir_upsampling_kernel)
pub struct FirFilter {
    /// FIR coefficients (already gain-scaled)
    h0: Vec<f64>,
    /// Delay line (lenh0 - 1 samples) — the canonical 1:1 FIR state.
    /// Serialized to Python as `state = t ++ [k0]`.
    /// For the OLA path this is the input history; the OLA output overlap
    /// is re-derivable from it on `set_state`.
    t: Vec<f64>,
    /// Down/up-sampling factor
    dwn_up: i64,
    /// 'D' = downsampling kernel, 'U' = upsampling kernel
    hswitch: char,
    /// Starting index for next block (downsampling only)
    k0: i64,
    /// Optional FFT overlap-add accelerator for the 1:1 path. Only
    /// populated when dwn_up == 1 and h0.len() >= OLA_THRESHOLD.
    ola: Option<OlaState>,
}

/// Threshold above which a 1:1 FIR switches to the FFT overlap-add path.
/// Below this, the direct convolution is faster (less FFT setup, no plan
/// overhead, similar work). Tuned by sweep in tests; see ADR in lib.rs
/// (module `ola_adaptive`).
pub const OLA_THRESHOLD: usize = 256;

/// Overlap-add state for the FFT fast path.
struct OlaState {
    /// Precomputed FFT of the kernel zero-padded to `fft_len`.
    h_fft: Vec<Complex<f64>>,
    /// Forward FFT plan (rustfft).
    fwd: Arc<dyn Fft<f64>>,
    /// Inverse FFT plan (rustfft).
    inv: Arc<dyn Fft<f64>>,
    /// Forward FFT plan (input is real, we use it twice: for X and for IFFT).
    fft_len: usize,
    /// Scratch work buffer (length fft_len, Complex).
    work: Vec<Complex<f64>>,
    /// Overlap-add carry from previous chunk: last `taps-1` output samples.
    overlap: Vec<f64>,
}

impl OlaState {
    /// Build an OlaState for a given (gain-scaled) coefficient set. The
    /// kernel itself is FFTed once and reused for every chunk.
    fn new(h0: &[f64]) -> Option<Self> {
        let taps = h0.len();
        // Target work-unit size per OLA round trip. 4096 is a good balance
        // for our kernel sizes: 8192 FFT for taps ≤ 592 (1 sub-call per
        // ~4 ms of audio) and 16384 FFT for taps up to 4001.
        const TARGET_CHUNK: usize = 4096;
        // The chunk is padded to fft_len, and the next call's overlap (length
        // taps - 1) must fit in the FFT output range. We therefore need
        //     chunk + (taps - 1) + (taps - 1) <= fft_len
        // i.e. fft_len >= TARGET_CHUNK + 2 * (taps - 1).
        let fft_len = (TARGET_CHUNK + 2 * (taps - 1)).next_power_of_two();
        let mut planner = FftPlanner::<f64>::new();
        let fwd = planner.plan_fft_forward(fft_len);
        let inv = planner.plan_fft_inverse(fft_len);
        // Precompute H = FFT(h0 zero-padded to fft_len).
        let mut h_fft: Vec<Complex<f64>> = h0.iter().map(|&h| Complex::new(h, 0.0)).collect();
        h_fft.resize(fft_len, Complex::new(0.0, 0.0));
        fwd.process(&mut h_fft);
        Some(Self {
            h_fft,
            fwd,
            inv,
            fft_len,
            work: vec![Complex::new(0.0, 0.0); fft_len],
            overlap: vec![0.0; taps.saturating_sub(1)],
        })
    }
}

impl FirFilter {
    /// Create FIR filter from coefficients and rate-change parameters.
    /// Selects the direct or OLA path based on (dwn_up, taps).
    pub fn new(h0: &[f64], gain: f64, dwn_up: i64, hswitch: char) -> Self {
        assert!(!h0.is_empty(), "Filter length must be > 0");
        let scaled: Vec<f64> = h0.iter().map(|&h| gain * h).collect();
        let ola = if dwn_up == 1 && hswitch == 'D' && scaled.len() >= OLA_THRESHOLD {
            OlaState::new(&scaled)
        } else {
            None
        };
        Self {
            h0: scaled,
            t: vec![0.0; h0.len() - 1],
            dwn_up,
            hswitch,
            k0: 0,
            ola,
        }
    }

    /// Reset filter state (clear delay line and phase counter)
    pub fn reset(&mut self) {
        self.t.fill(0.0);
        self.k0 = 0;
        if let Some(ola) = self.ola.as_mut() {
            ola.overlap.fill(0.0);
        }
    }

    /// Serialize filter state into a flat byte vector (for Python interop).
    ///
    /// The canonical 1:1 state is the input history (`t` = last
    /// `lenh0-1` input samples). For OLA, the overlap-carry is *derivable*
    /// from this history by replaying the direct convolution on it, so we
    /// do not serialize the overlap separately. Cross-path roundtrip:
    /// an old (direct-only) state is valid in a new OLA filter, and a
    /// new OLA state is valid in a direct filter.
    pub fn get_state(&self) -> Vec<f64> {
        let mut state = self.t.clone();
        state.push(self.k0 as f64);
        state
    }

    /// Restore filter state from a flat vector. Reconstructs the OLA
    /// overlap by a single O(lenh0) direct replay on the input history.
    pub fn set_state(&mut self, state: &[f64]) {
        let lenh0 = self.h0.len();
        let expected = lenh0 - 1 + 1; // t + k0
        if state.len() >= expected {
            self.t.copy_from_slice(&state[..lenh0 - 1]);
            self.k0 = state[lenh0 - 1] as i64;
        }
        if let Some(ola) = self.ola.as_mut() {
            // Re-derive the OLA overlap from the input history. With no chunk
            // yet, the next call's history IS self.t. The overlap is the
            // linear-convolution spillover of (self.t, empty_chunk) with the
            // kernel — which is just y_lin[history_len .. 2*history_len].
            // Equivalently, the part of y_out that depends on history
            // (not on the next chunk) is:
            //   overlap[i] = sum_{k=i+1..=taps-1} h[k] * self.t[history_len + i - k]
            let history_len = self.t.len();
            for i in 0..ola.overlap.len() {
                let mut acc = 0.0;
                for k in (i + 1)..=self.h0.len() - 1 {
                    acc += self.h0[k] * self.t[history_len + i - k];
                }
                ola.overlap[i] = acc;
            }
        }
    }

    /// Process a block of samples; returns output samples
    pub fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        if self.hswitch == 'U' {
            self.upsampling_kernel(x)
        } else if self.ola.is_some() {
            self.ola_process_block(x)
        } else {
            self.downsampling_kernel(x)
        }
    }

    /// FFT overlap-add fast path for the 1:1 FIR case.
    ///
    /// Per call:
    ///   1. x_in = [history (=self.t) ++ chunk]
    ///   2. y_full = Re(IFFT(FFT(x_padded) * H))
    ///   3. y_out = y_full[0..chunk.len()] + overlap_carry  (and store the
    ///      spillover back into `overlap`)
    ///   4. history = chunk[taps-1-len..]  (last taps-1 of the chunk)
    ///   5. overlap = y_full[chunk.len() .. chunk.len() + taps - 1]
    ///
    /// Equivalence: this is the canonical overlap-add algorithm and is
    /// exact up to f64 rounding (same polynomial product as the direct
    /// path), so the G.191 conformance margin (±1 int16) is comfortably
    /// met.
    ///
    /// If the input is larger than the FFT work-unit (`fft_len - history_len`),
    /// it is processed in sub-blocks that each fit one OLA round trip.
    fn ola_process_block(&mut self, x: &[f64]) -> Vec<f64> {
        let ola = self.ola.as_mut().expect("OLA path selected without OlaState");
        let fft_len = ola.fft_len;
        let history_len = self.t.len();

        // The largest chunk that fits in one FFT round-trip is fft_len minus
        // the history (we prepend the history to the chunk). Recurse if the
        // caller's chunk is larger.
        let max_chunk = fft_len - history_len;
        if x.len() > max_chunk {
            let mut y = Vec::with_capacity(x.len());
            for chunk in x.chunks(max_chunk) {
                y.extend(self.ola_process_block(chunk));
            }
            return y;
        }

        if x.is_empty() {
            return Vec::new();
        }

        // 1. Compose x_padded = history ++ chunk ++ zeros. Reuse `ola.work`
        //    (a Complex buffer). Map to the real parts; imaginary parts are
        //    zero because the input is real.
        for z in ola.work.iter_mut() {
            *z = Complex::new(0.0, 0.0);
        }
        for (i, &s) in self.t.iter().enumerate() {
            ola.work[i] = Complex::new(s, 0.0);
        }
        for (i, &s) in x.iter().enumerate() {
            ola.work[history_len + i] = Complex::new(s, 0.0);
        }

        // 2. FFT -> multiply by H -> IFFT.
        ola.fwd.process(&mut ola.work);
        for (z, h) in ola.work.iter_mut().zip(ola.h_fft.iter()) {
            *z *= *h;
        }
        ola.inv.process(&mut ola.work);
        // Normalize inverse FFT (rustfft IFFT does not normalize).
        let inv_n = 1.0 / fft_len as f64;
        for z in ola.work.iter_mut() {
            *z *= inv_n;
        }
        let y_full: Vec<f64> = ola.work.iter().map(|c| c.re).collect();

        // 3. Emit: y_out[j] = y_full[history_len + j] for j in 0..chunk_len.
        //    The history is already in x_in (prepended to the chunk), so
        //    y_full[history_len + j] is the full linear convolution at the
        //    chunk-relative position j, including all history contributions.
        //    No +overlap: the previous call's overlap was the "old sample"
        //    contribution from samples beyond the current call's history,
        //    but for the first history_len positions of y_out those samples
        //    are not needed (the current history covers them).
        let y_out: Vec<f64> = (0..x.len()).map(|j| y_full[history_len + j]).collect();

        // 4. Update overlap: the spillover y_full[history_len + chunk_len..]
        //    of length up to history_len, all of which is in [0..fft_len-1]
        //    by the chunking guard above.
        let spill_base = history_len + x.len();
        for (i, dst) in ola.overlap.iter_mut().enumerate() {
            let idx = spill_base + i;
            if idx < fft_len {
                *dst = y_full[idx];
            }
            // else: keep prior value (only possible if the input was
            // very short and the spill fell past fft_len, which the
            // chunking guard prevents; left as a defensive no-op).
        }

        // 5. Update history (delay line) with the tail of the chunk.
        if x.len() >= history_len {
            for i in 0..history_len {
                self.t[i] = x[x.len() - history_len + i];
            }
        } else {
            // Shift history left by `x.len()`, append the chunk at the tail.
            for i in 0..(history_len - x.len()) {
                self.t[i] = self.t[i + x.len()];
            }
            for i in 0..x.len() {
                self.t[history_len - x.len() + i] = x[i];
            }
        }

        y_out
    }

    /// STL fir_downsampling_kernel
    fn downsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let lenx = x.len();
        if lenx == 0 {
            return Vec::new();
        }
        let lenh0 = self.h0.len();
        let downfac = self.dwn_up as usize;
        let mut y = Vec::new();

        // First Step: transition from k=0..lenh0-2
        let mut kstart = self.k0 as usize;
        let mut ktrans = lenh0 - 1;
        if ktrans > lenx - 1 {
            ktrans = lenx - 1;
        }

        let mut kx = self.k0 as usize;
        while kx <= ktrans {
            let mut acc = x[kx] * self.h0[0];
            for kappa in 1..=kx {
                acc += x[kx - kappa] * self.h0[kappa];
            }
            for kappa in (kx + 1)..lenh0 {
                acc += self.t[lenh0 - 2 + kx + 1 - kappa] * self.h0[kappa];
            }
            y.push(acc);
            kstart = kx;
            kx += downfac;
        }

        // Second Step: remaining part in x-array
        self.k0 = kstart as i64;
        let mut kx = kstart + downfac;
        while kx < lenx {
            let mut acc = x[kx] * self.h0[0];
            for kappa in 1..lenh0 {
                acc += x[kx - kappa] * self.h0[kappa];
            }
            y.push(acc);
            self.k0 = kx as i64;
            kx += downfac;
        }

        // Update k0 for next block
        if self.k0 <= (lenx - 1) as i64 {
            self.k0 = self.k0 + downfac as i64 - lenx as i64;
        } else {
            self.k0 -= lenx as i64;
        }

        // Last Step: copy end of x-array into T-array (update delay line)
        // C reference stores delay line chronologically (oldest first).
        if lenx >= lenh0 - 1 {
            for kappa in 0..(lenh0 - 1) {
                self.t[kappa] = x[lenx + 1 - lenh0 + kappa];
            }
        } else {
            // Left-Shift of T-array
            for kappa in 0..(lenh0 - 1 - lenx) {
                self.t[kappa] = self.t[kappa + lenx];
            }
            // Copy complete x-array -> T-array
            for kappa in (lenh0 - 1 - lenx)..(lenh0 - 1) {
                self.t[kappa] = x[lenx - 1 + kappa - (lenh0 - 2)];
            }
        }

        y
    }

    /// STL fir_upsampling_kernel: filter + up-by-dwn_up (outputs dwn_up samples per input).
    fn upsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let lenx = x.len();
        if lenx == 0 {
            return Vec::new();
        }
        let lenh0 = self.h0.len();
        let iupfac = self.dwn_up as usize;
        let lh = lenh0 / iupfac;        /* = lenh0 / iupfac in C */
        let lh_1 = lh - 1;            /* lenh0/iupfac - 1 (C upper bound; iupfac<=lenh0/2 so lh>=2) */
        let mut y = Vec::new();

        /* ............. FIRST STEP: transition from k=0..lh-2 ............. */
        let ktrans = if lh > lenx { lenx } else { lh };
        let mut kstart = 0usize;
        for kx in 0..ktrans {
            for iup in 0..iupfac {
                let mut acc = x[kx] * self.h0[iup];
                for kappa in 1..=kx {
                    acc += x[kx - kappa] * self.h0[iup + kappa * iupfac];
                }
                for kappa in (kx + 1)..lh {
                    acc += self.t[lh - 2 + kx + 1 - kappa] * self.h0[iup + kappa * iupfac];
                }
                y.push(acc);
            }
            kstart = kx;
        }

        /* ............. SECOND STEP: remaining dot-products from x[] ............. */
        for kx in (kstart + 1)..lenx {
            for iup in 0..iupfac {
                let mut acc = x[kx] * self.h0[iup];
                for kappa in 1..lh {
                    acc += x[kx - kappa] * self.h0[iup + kappa * iupfac];
                }
                y.push(acc);
            }
        }

        /* ............. LAST STEP: update delay line ............. */
        /* C: T[kappa] = x[lenx+1-lh+kappa], kappa = 0..<=lh-2 (== 0..lh_1) */
        if lenx >= lh_1 {
            for kappa in 0..lh_1 {
                self.t[kappa] = x[lenx + 1 - lh + kappa];
            }
        } else {
            /* left-shift T[k] = T[k+lenx]; kappa = 0..<=lh-2-lenx (== 0..lh_1-lenx) */
            for kappa in 0..(lh_1 - lenx) {
                self.t[kappa] = self.t[kappa + lenx];
            }
            /* copy end of x[] -> T[]; kappa = lh-1-lenx .. lh-2 (== lh_1-lenx .. lh_1) */
            for kappa in (lh_1 - lenx)..lh_1 {
                self.t[kappa] = x[lenx - 1 + kappa - (lh - 2)];
            }
        }

        y
    }
}
