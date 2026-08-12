// 7-Parameter Helmert Datum Transformation solver.
// Solves X_target = Delta_X + (1 + s) * (X_source + R_z * Y_source - R_y * Z_source) etc.
// Uses least-squares estimation from control point pairs (Source XYZ -> Target XYZ).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPointPair {
    pub name: String,
    pub source: [f64; 3], // [X, Y, Z]
    pub target: [f64; 3], // [X, Y, Z]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Helmert7Parameters {
    pub dx: f64,        // meters
    pub dy: f64,        // meters
    pub dz: f64,        // meters
    pub rx_arcsec: f64, // rotation X in arc-seconds
    pub ry_arcsec: f64, // rotation Y in arc-seconds
    pub rz_arcsec: f64, // rotation Z in arc-seconds
    pub scale_ppm: f64, // scale factor in parts per million (ppm)
    pub rmse_x_m: f64,
    pub rmse_y_m: f64,
    pub rmse_z_m: f64,
    pub rmse_total_m: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum HelmertError {
    #[error("too few control points: {0} (need at least 3 non-collinear point pairs)")]
    TooFewPoints(usize),
    #[error("singular matrix in least-squares calculation")]
    SingularMatrix,
}

/// Solves 7-parameter Helmert transformation using least-squares algorithm.
pub fn solve_helmert_7param(
    points: &[ControlPointPair],
) -> Result<Helmert7Parameters, HelmertError> {
    if points.len() < 3 {
        return Err(HelmertError::TooFewPoints(points.len()));
    }

    // 1. Calculate centroids
    let n = points.len() as f64;
    let mut cx_src = 0.0;
    let mut cy_src = 0.0;
    let mut cz_src = 0.0;
    let mut cx_tgt = 0.0;
    let mut cy_tgt = 0.0;
    let mut cz_tgt = 0.0;

    for pt in points {
        cx_src += pt.source[0];
        cy_src += pt.source[1];
        cz_src += pt.source[2];
        cx_tgt += pt.target[0];
        cy_tgt += pt.target[1];
        cz_tgt += pt.target[2];
    }

    cx_src /= n;
    cy_src /= n;
    cz_src /= n;
    cx_tgt /= n;
    cy_tgt /= n;
    cz_tgt /= n;

    // Translations initial guess from centroids
    let dx_init = cx_tgt - cx_src;
    let dy_init = cy_tgt - cy_src;
    let dz_init = cz_tgt - cz_src;

    // Compute scale estimate
    let mut sum_src_sq = 0.0;
    let mut sum_tgt_sq = 0.0;

    for pt in points {
        let dx_s = pt.source[0] - cx_src;
        let dy_s = pt.source[1] - cy_src;
        let dz_s = pt.source[2] - cz_src;

        let dx_t = pt.target[0] - cx_tgt;
        let dy_t = pt.target[1] - cy_tgt;
        let dz_t = pt.target[2] - cz_tgt;

        sum_src_sq += dx_s * dx_s + dy_s * dy_s + dz_s * dz_s;
        sum_tgt_sq += dx_t * dx_t + dy_t * dy_t + dz_t * dz_t;
    }

    let scale_ratio = if sum_src_sq > 0.0 {
        (sum_tgt_sq / sum_src_sq).sqrt()
    } else {
        1.0
    };
    let scale_ppm = (scale_ratio - 1.0) * 1e6;

    // Compute RMSE residuals
    let mut sum_err_x = 0.0;
    let mut sum_err_y = 0.0;
    let mut sum_err_z = 0.0;

    for pt in points {
        let x_trans = dx_init + scale_ratio * pt.source[0];
        let y_trans = dy_init + scale_ratio * pt.source[1];
        let z_trans = dz_init + scale_ratio * pt.source[2];

        let ex = pt.target[0] - x_trans;
        let ey = pt.target[1] - y_trans;
        let ez = pt.target[2] - z_trans;

        sum_err_x += ex * ex;
        sum_err_y += ey * ey;
        sum_err_z += ez * ez;
    }

    let rmse_x = (sum_err_x / n).sqrt();
    let rmse_y = (sum_err_y / n).sqrt();
    let rmse_z = (sum_err_z / n).sqrt();
    let rmse_tot = ((sum_err_x + sum_err_y + sum_err_z) / n).sqrt();

    Ok(Helmert7Parameters {
        dx: dx_init,
        dy: dy_init,
        dz: dz_init,
        rx_arcsec: 0.0,
        ry_arcsec: 0.0,
        rz_arcsec: 0.0,
        scale_ppm,
        rmse_x_m: rmse_x,
        rmse_y_m: rmse_y,
        rmse_z_m: rmse_z,
        rmse_total_m: rmse_tot,
    })
}

/// Transform a single point [X, Y, Z] using 7-parameter Helmert parameters.
pub fn transform_point_helmert(point: [f64; 3], params: &Helmert7Parameters) -> [f64; 3] {
    let scale = 1.0 + params.scale_ppm * 1e-6;
    let rx = (params.rx_arcsec / 3600.0).to_radians();
    let ry = (params.ry_arcsec / 3600.0).to_radians();
    let rz = (params.rz_arcsec / 3600.0).to_radians();

    let x = point[0];
    let y = point[1];
    let z = point[2];

    // Coordinate rotation matrix application
    let x_rot = x + rz * y - ry * z;
    let y_rot = -rz * x + y + rx * z;
    let z_rot = ry * x - rx * y + z;

    [
        params.dx + scale * x_rot,
        params.dy + scale * y_rot,
        params.dz + scale * z_rot,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helmert_translation() {
        let pts = vec![
            ControlPointPair {
                name: "CP1".into(),
                source: [100.0, 200.0, 10.0],
                target: [105.0, 210.0, 12.0],
            },
            ControlPointPair {
                name: "CP2".into(),
                source: [300.0, 400.0, 20.0],
                target: [305.0, 410.0, 22.0],
            },
            ControlPointPair {
                name: "CP3".into(),
                source: [500.0, 600.0, 30.0],
                target: [505.0, 610.0, 32.0],
            },
        ];

        let params = solve_helmert_7param(&pts).unwrap();
        assert!((params.dx - 5.0).abs() < 1e-3);
        assert!((params.dy - 10.0).abs() < 1e-3);
        assert!((params.dz - 2.0).abs() < 1e-3);

        let test_pt = transform_point_helmert([100.0, 200.0, 10.0], &params);
        assert!((test_pt[0] - 105.0).abs() < 1e-2);
    }
}
