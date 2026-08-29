// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

/// FIR filter implementation matching STL reference algorithms
/// (fir-lib.c: fir_initialization, fir_downsampling_kernel, fir_upsampling_kernel)

pub struct FirFilter {
    /// FIR coefficients (already gain-scaled)
    h0: Vec<f64>,
    /// Delay line (lenh0 - 1 samples)
    t: Vec<f64>,
    /// Down/up-sampling factor
    dwn_up: i64,
    /// 'D' = downsampling kernel, 'U' = upsampling kernel
    hswitch: char,
    /// Starting index for next block (downsampling only)
    k0: i64,
}

impl FirFilter {
    /// Create FIR filter from coefficients and rate-change parameters
    pub fn new(h0: &[f64], gain: f64, dwn_up: i64, hswitch: char) -> Self {
        assert!(!h0.is_empty(), "Filter length must be > 0");
        let scaled: Vec<f64> = h0.iter().map(|&h| gain * h).collect();
        Self {
            h0: scaled,
            t: vec![0.0; h0.len() - 1],
            dwn_up,
            hswitch,
            k0: 0,
        }
    }

    /// Reset filter state (clear delay line and phase counter)
    pub fn reset(&mut self) {
        self.t.fill(0.0);
        self.k0 = 0;
    }

    /// Serialize filter state into a flat byte vector (for Python interop)
    pub fn get_state(&self) -> Vec<f64> {
        let mut state = self.t.clone();
        state.push(self.k0 as f64);
        state
    }

    /// Restore filter state from a flat vector
    pub fn set_state(&mut self, state: &[f64]) {
        let lenh0 = self.h0.len();
        let expected = lenh0 - 1 + 1; // t + k0
        if state.len() >= expected {
            self.t.copy_from_slice(&state[..lenh0 - 1]);
            self.k0 = state[lenh0 - 1] as i64;
        }
    }

    /// Process a block of samples; returns output samples
    pub fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        if self.hswitch == 'U' {
            self.upsampling_kernel(x)
        } else {
            self.downsampling_kernel(x)
        }
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
        while kx <= lenx - 1 {
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

    /// STL fir_upsampling_kernel
    fn upsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let lenx = x.len();
        if lenx == 0 {
            return Vec::new();
        }
        let lenh0 = self.h0.len();
        let iupfac = self.dwn_up as usize;
        let mut y = Vec::new();

        // First Step: transition from k=0..lenh0/iupfac-2
        let ktrans = (lenh0 / iupfac).min(lenx);
        let mut kstart = 0usize;

        for kx in 0..ktrans {
            for iup in 0..iupfac {
                let mut acc = x[kx] * self.h0[iup];
                for kappa in 1..=kx {
                    acc += x[kx - kappa] * self.h0[iup + kappa * iupfac];
                }
                for kappa in (kx + 1)..(lenh0 / iupfac) {
                    acc += self.t[lenh0 / iupfac - 2 + kx + 1 - kappa]
                        * self.h0[iup + kappa * iupfac];
                }
                y.push(acc);
            }
            kstart = kx;
        }

        // Second Step: remaining dot-products completely from x[]
        for kx in (kstart + 1)..lenx {
            for iup in 0..iupfac {
                let mut acc = x[kx] * self.h0[iup];
                for kappa in 1..(lenh0 / iupfac) {
                    acc += x[kx - kappa] * self.h0[iup + kappa * iupfac];
                }
                y.push(acc);
            }
        }

        // Last Step: update delay line
        // C reference stores delay line chronologically (oldest first).
        if lenx >= lenh0 / iupfac - 1 {
            for kappa in 0..(lenh0 / iupfac) {
                self.t[kappa] = x[lenx + 1 - lenh0 / iupfac + kappa];
            }
        } else {
            let shift = lenh0 / iupfac - 1 - lenx;
            for kappa in 0..shift {
                self.t[kappa] = self.t[kappa + lenx];
            }
            for kappa in (lenh0 / iupfac - 1 - lenx)..(lenh0 / iupfac) {
                self.t[kappa] = x[lenx - 1 + kappa - (lenh0 / iupfac - 2)];
            }
        }

        y
    }
}
