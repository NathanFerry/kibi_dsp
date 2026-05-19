pub struct BodePlot {
    pub frequencies: Vec<f32>,
    pub magnitude_db: Vec<f32>,
    pub phase_deg: Vec<f32>,
}

pub fn compute_bode(_b: &[f32], _a: &[f32], _sample_rate: f32, _n_points: usize) -> BodePlot {
    todo!()
}
