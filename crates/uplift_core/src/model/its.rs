use nalgebra::{DMatrix, DVector};

use crate::error::{Error, Result};
use crate::timeseries::TimeSeries;

const MIN_PRE_PERIOD: usize = 30;
const FOURIER_PERIOD: f64 = 7.0;

pub struct ItsModel {
    coefficients: DVector<f64>,
    residuals: Vec<f64>,
    pre_period_len: usize,
}

fn build_design_matrix(n: usize, t_offset: usize) -> DMatrix<f64> {
    let two_pi = 2.0 * std::f64::consts::PI;
    DMatrix::from_fn(n, 6, |row, col| -> f64 {
        let t = (row + t_offset) as f64;
        match col {
            0 => 1.0,
            1 => t,
            2 => (two_pi * t / FOURIER_PERIOD).sin(),
            3 => (two_pi * t / FOURIER_PERIOD).cos(),
            4 => (2.0 * two_pi * t / FOURIER_PERIOD).sin(),
            5 => (2.0 * two_pi * t / FOURIER_PERIOD).cos(),
            _ => {
                unreachable!()
            }
        }
    })
}

impl ItsModel {
    pub fn fit(pre_period: &TimeSeries) -> Result<Self> {
        let n = pre_period.len();
        if n < MIN_PRE_PERIOD {
            return Err(Error::InsufficientData {
                min: MIN_PRE_PERIOD,
                got: n,
            });
        }

        let x = build_design_matrix(n, 0);
        let y = DVector::from_vec(pre_period.values().collect::<Vec<_>>());

        let xtx = x.transpose() * &x;
        let xty = x.transpose() * &y;

        let coefficients = xtx
            .cholesky()
            .ok_or_else(|| Error::ModelFitFailed("XᵀX is not positive definite".into()))?
            .solve(&xty);

        let fitted = &x * &coefficients;
        let residuals = (0..n).map(|i| y[i] - fitted[i]).collect();

        Ok(Self {
            coefficients,
            residuals,
            pre_period_len: n,
        })
    }

    pub fn predict(&self, n_post: usize) -> Vec<f64> {
        let x = build_design_matrix(n_post, self.pre_period_len);
        (x * &self.coefficients).iter().cloned().collect()
    }

    pub fn residuals(&self) -> &[f64] {
        &self.residuals
    }
}
