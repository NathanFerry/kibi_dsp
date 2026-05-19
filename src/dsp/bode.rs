pub struct BodePlot {
    pub frequencies: Vec<f32>,
    pub magnitude_db: Vec<f32>,
    pub phase_deg: Vec<f32>,
}

/// Evaluate H(e^jw) = B(e^jw) / A(e^jw) at `n_points` log-spaced frequencies
/// from 20 Hz to `sample_rate / 2`.
pub fn compute_bode(b: &[f32], a: &[f32], sample_rate: f32, n_points: usize) -> BodePlot {
    let f_min = 20.0_f32;
    let f_max = (sample_rate / 2.0).min(20_000.0);

    let mut frequencies = Vec::with_capacity(n_points);
    let mut magnitude_db = Vec::with_capacity(n_points);
    let mut phase_deg = Vec::with_capacity(n_points);

    for i in 0..n_points {
        let t = i as f32 / (n_points - 1).max(1) as f32;
        let f = f_min * (f_max / f_min).powf(t);
        let w = 2.0 * std::f32::consts::PI * f / sample_rate;

        // B(e^jw) = sum_k b[k] * e^{-j w k}
        let (mut b_re, mut b_im) = (0.0_f32, 0.0_f32);
        for (k, &bk) in b.iter().enumerate() {
            let (s, c) = (-(w * k as f32)).sin_cos();
            b_re += bk * c;
            b_im += bk * s;
        }

        // A(e^jw) = sum_k a[k] * e^{-j w k}
        let (mut a_re, mut a_im) = (0.0_f32, 0.0_f32);
        for (k, &ak) in a.iter().enumerate() {
            let (s, c) = (-(w * k as f32)).sin_cos();
            a_re += ak * c;
            a_im += ak * s;
        }

        // H = B / A  (complex division)
        let denom = a_re * a_re + a_im * a_im;
        let (h_re, h_im) = if denom > 1e-12 {
            (
                (b_re * a_re + b_im * a_im) / denom,
                (b_im * a_re - b_re * a_im) / denom,
            )
        } else {
            (0.0, 0.0)
        };

        let mag = (h_re * h_re + h_im * h_im).sqrt().max(1e-6);
        frequencies.push(f);
        magnitude_db.push(20.0 * mag.log10());
        phase_deg.push(h_im.atan2(h_re).to_degrees());
    }

    BodePlot {
        frequencies,
        magnitude_db,
        phase_deg,
    }
}
