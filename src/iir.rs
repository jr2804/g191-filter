// SPDX-License-Identifier: MIT
// Copyright 2026, Jan.Reimes

//! IIR filter implementations matching STL reference algorithms
//! (iir-lib.c: stdpcm_kernel, cascade_form_iir_*_kernel, direct_form_iir_*_kernel)

/// Parallel-form IIR filter (STL stdpcm / G.712)
pub struct IirFilter {
    /// Direct path coefficient
    direct_cof: f64,
    /// Gain factor
    gain: f64,
    /// Numerator coefficients b[n][3]
    b: Vec<[f64; 3]>,
    /// Denominator coefficients c[n][2]
    c: Vec<[f64; 2]>,
    /// State variables T[n][2]
    t: Vec<[f64; 2]>,
    /// Down/up-sampling factor
    idown: i64,
    /// Phase counter (down-sampling only; unused for up-sampling)
    k0: i64,
    /// true = up-sampling kernel, false = down-sampling kernel
    is_up: bool,
}

impl IirFilter {
    /// Create parallel-form IIR filter
    pub fn new(gain: f64, direct_cof: f64, b: &[[f64; 3]], c: &[[f64; 2]], idown: i64, is_up: bool) -> Self {
        let nblocks = b.len();
        Self { direct_cof, gain, b: b.to_vec(), c: c.to_vec(), t: vec![[0.0; 2]; nblocks], idown, k0: 0, is_up }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.t.iter_mut().for_each(|row| *row = [0.0; 2]);
        self.k0 = 0;
    }

    /// Serialize filter state
    pub fn get_state(&self) -> Vec<f64> {
        let mut state = Vec::with_capacity(self.t.len() * 2 + 1);
        for row in &self.t { state.push(row[0]); state.push(row[1]); }
        state.push(self.k0 as f64);
        state
    }

    /// Restore filter state
    pub fn set_state(&mut self, state: &[f64]) {
        let nblocks = self.t.len();
        let expected = nblocks * 2 + 1;
        if state.len() >= expected {
            for n in 0..nblocks { self.t[n][0] = state[n * 2]; self.t[n][1] = state[n * 2 + 1]; }
            self.k0 = state[nblocks * 2] as i64;
        }
    }

    /// Process a block of samples (dispatches to up or down kernel)
    pub fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        if self.is_up { self.upsampling_kernel(x) } else { self.downsampling_kernel(x) }
    }

    /// STL scd_parallel_form_iir_up_kernel
    fn upsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let lenx = x.len();
        let nblocks = self.b.len();
        let iup = self.idown as usize;
        let mut y = Vec::with_capacity(iup * lenx);
        let mut kx = 0usize;
        for ky in 0..(iup * lenx) {
            let xj = if ky % iup == 0 { kx += 1; x[kx - 1] } else { 0.0 };
            let mut acc = self.direct_cof * xj;
            for n in 0..nblocks {
                let ttmp = 2.0 * (xj - self.c[n][0] * self.t[n][0] - self.c[n][1] * self.t[n][1]);
                acc += self.b[n][2] * ttmp + self.b[n][1] * self.t[n][1] + self.b[n][0] * self.t[n][0];
                self.t[n][0] = self.t[n][1];
                self.t[n][1] = ttmp;
            }
            y.push(acc * self.gain);
        }
        y
    }

    /// STL scd_parallel_form_iir_down_kernel
    fn downsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let nblocks = self.b.len();
        let mut y = Vec::new();
        for &xi in x {
            if self.k0 < self.idown {
                let mut acc = self.direct_cof * xi;
                for n in 0..nblocks {
                    let ttmp = 2.0 * (xi - self.c[n][0] * self.t[n][0] - self.c[n][1] * self.t[n][1]);
                    acc += self.b[n][2] * ttmp + self.b[n][1] * self.t[n][1] + self.b[n][0] * self.t[n][0];
                    self.t[n][0] = self.t[n][1];
                    self.t[n][1] = ttmp;
                }
                y.push(acc * self.gain);
            } else {
                for n in 0..nblocks {
                    let ttmp = 2.0 * (xi - self.c[n][0] * self.t[n][0] - self.c[n][1] * self.t[n][1]);
                    self.t[n][0] = self.t[n][1];
                    self.t[n][1] = ttmp;
                }
            }
            self.k0 += 1;
        }
        self.k0 %= self.idown;
        y
    }
}

/// Cascade-form IIR filter (STL cascade_form_iir_*_kernel)
pub struct CascadeIirFilter {
    /// Gain factor
    gain: f64,
    /// Numerator coefficients a[n][2] (feedforward on x)
    a: Vec<[f64; 2]>,
    /// Denominator coefficients b[n][2] (feedback on y)
    b: Vec<[f64; 2]>,
    /// State variables T[n][4]: [x-1, x-2, y-1, y-2]
    t: Vec<[f64; 4]>,
    /// Down/up-sampling factor
    idown: i64,
    /// Phase counter (down-sampling only; unused for up-sampling)
    k0: i64,
    /// true = up-sampling kernel, false = down-sampling kernel
    is_up: bool,
}

impl CascadeIirFilter {
    /// Create cascade-form IIR filter
    pub fn new(gain: f64, a: &[[f64; 2]], b: &[[f64; 2]], idown: i64, is_up: bool) -> Self {
        let nblocks = a.len();
        Self { gain, a: a.to_vec(), b: b.to_vec(), t: vec![[0.0; 4]; nblocks], idown, k0: 0, is_up }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.t.iter_mut().for_each(|row| *row = [0.0; 4]);
        self.k0 = 0;
    }

    /// Serialize filter state
    pub fn get_state(&self) -> Vec<f64> {
        let mut state = Vec::with_capacity(self.t.len() * 4 + 1);
        for row in &self.t { state.extend_from_slice(row); }
        state.push(self.k0 as f64);
        state
    }

    /// Restore filter state
    pub fn set_state(&mut self, state: &[f64]) {
        let nblocks = self.t.len();
        let expected = nblocks * 4 + 1;
        if state.len() >= expected {
            for n in 0..nblocks {
                self.t[n][0] = state[n * 4];
                self.t[n][1] = state[n * 4 + 1];
                self.t[n][2] = state[n * 4 + 2];
                self.t[n][3] = state[n * 4 + 3];
            }
            self.k0 = state[nblocks * 4] as i64;
        }
    }

    /// Process a block of samples (dispatches to up or down kernel)
    pub fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        if self.is_up { self.upsampling_kernel(x) } else { self.downsampling_kernel(x) }
    }

    /// STL cascade_form_iir_up_kernel
    fn upsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let nblocks = self.a.len();
        let iup = self.idown as usize;
        let mut y = Vec::with_capacity(iup * x.len());
        let mut kx = 0usize;
        for ky in 0..(iup * x.len()) {
            let xj = if ky % iup == 0 { kx += 1; x[kx - 1] } else { 0.0 };
            let mut xj_buf = xj;
            for n in 0..nblocks {
                let yj = xj_buf + self.a[n][0] * self.t[n][0] + self.a[n][1] * self.t[n][1]
                    - (self.b[n][0] * self.t[n][2] + self.b[n][1] * self.t[n][3]);
                self.t[n][1] = self.t[n][0];
                self.t[n][0] = xj_buf;
                self.t[n][3] = self.t[n][2];
                self.t[n][2] = yj;
                xj_buf = yj;
            }
            y.push(xj_buf * self.gain);
        }
        y
    }

    /// STL cascade_form_iir_down_kernel
    fn downsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let nblocks = self.a.len();
        let mut y = Vec::new();
        for &xi in x {
            let mut xj = xi;
            let mut yj = 0.0;
            for n in 0..nblocks {
                yj = xj + self.a[n][0] * self.t[n][0] + self.a[n][1] * self.t[n][1]
                    - (self.b[n][0] * self.t[n][2] + self.b[n][1] * self.t[n][3]);
                self.t[n][1] = self.t[n][0];
                self.t[n][0] = xj;
                self.t[n][3] = self.t[n][2];
                self.t[n][2] = yj;
                xj = yj;
            }
            y.push(yj * self.gain);
        }
        y
    }
}

/// Direct-form IIR filter (STL direct_form_iir_*_kernel)
pub struct DirectIirFilter {
    /// Gain factor
    gain: f64,
    /// Numerator coefficients a[n]
    a: Vec<f64>,
    /// Denominator coefficients b[n] (b[0]=1.0)
    b: Vec<f64>,
    /// State variables T[n][2]
    t: Vec<[f64; 2]>,
    /// Down/up-sampling factor
    idown: i64,
    /// Phase counter
    k0: i64,
    /// true = up-sampling kernel, false = down-sampling kernel
    is_up: bool,
}

impl DirectIirFilter {
    /// Create direct-form IIR filter
    pub fn new(gain: f64, a: &[f64], b: &[f64], idown: i64, is_up: bool) -> Self {
        Self { gain, a: a.to_vec(), b: b.to_vec(), t: vec![[0.0; 2]; a.len().max(b.len())], idown, k0: 0, is_up }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.t.iter_mut().for_each(|row| *row = [0.0; 2]);
        self.k0 = 0;
    }

    /// Serialize filter state
    pub fn get_state(&self) -> Vec<f64> {
        let mut state = Vec::with_capacity(self.t.len() * 2 + 1);
        for row in &self.t { state.push(row[0]); state.push(row[1]); }
        state.push(self.k0 as f64);
        state
    }

    /// Restore filter state
    pub fn set_state(&mut self, state: &[f64]) {
        let n = self.t.len();
        let expected = n * 2 + 1;
        if state.len() >= expected {
            for i in 0..n { self.t[i][0] = state[i * 2]; self.t[i][1] = state[i * 2 + 1]; }
            self.k0 = state[n * 2] as i64;
        }
    }

    /// Process a block of samples (dispatches to up or down kernel)
    pub fn process_block(&mut self, x: &[f64]) -> Vec<f64> {
        if self.is_up { self.upsampling_kernel(x) } else { self.downsampling_kernel(x) }
    }

    /// STL direct_form_iir_up_kernel
    fn upsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let poleno = self.b.len();
        let zerono = self.a.len();
        let iup = self.idown as usize;
        let mut y = Vec::with_capacity(iup * x.len());
        let mut kx = 0usize;
        for ky in 0..(iup * x.len()) {
            let xj = if ky % iup == 0 { kx += 1; x[kx - 1] } else { 0.0 };
            self.t[0][0] = xj;
            let mut yj = 0.0;
            for n in 0..zerono { yj += self.a[n] * self.t[n][0]; }
            for n in 1..poleno { yj -= self.b[n] * self.t[n - 1][1]; }
            for n in (1..zerono).rev() { self.t[n][0] = self.t[n - 1][0]; }
            for n in (1..poleno).rev() { self.t[n][1] = self.t[n - 1][1]; }
            self.t[0][1] = yj;
            y.push(yj * self.gain);
        }
        y
    }

    /// STL direct_form_iir_down_kernel
    fn downsampling_kernel(&mut self, x: &[f64]) -> Vec<f64> {
        let poleno = self.b.len();
        let zerono = self.a.len();
        let mut y = Vec::new();
        for &xi in x {
            self.t[0][0] = xi;
            let mut yj = 0.0;
            for n in 0..zerono { yj += self.a[n] * self.t[n][0]; }
            for n in 1..poleno { yj -= self.b[n] * self.t[n - 1][1]; }
            for n in (1..zerono).rev() { self.t[n][0] = self.t[n - 1][0]; }
            for n in (1..poleno).rev() { self.t[n][1] = self.t[n - 1][1]; }
            self.t[0][1] = yj;
            if self.k0 % self.idown == 0 { y.push(yj * self.gain); }
            self.k0 += 1;
        }
        self.k0 %= self.idown;
        y
    }
}
