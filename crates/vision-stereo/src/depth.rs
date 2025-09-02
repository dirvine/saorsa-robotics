#[cfg(feature = "opencv")]
use crate::calib::CalibrationStereo;
#[cfg(feature = "opencv")]
use crate::{Error, Result};
#[cfg(feature = "opencv")]
use opencv::{calib3d, core, imgcodecs, prelude::*};

#[cfg(feature = "opencv")]
#[derive(Debug, Copy, Clone)]
pub struct StereoSgbmParams {
    pub min_disp: i32,
    pub num_disp: i32,
    pub block_size: i32,
    pub uniqueness_ratio: i32,
    pub speckle_window_size: i32,
    pub speckle_range: i32,
    pub disp12_max_diff: i32,
    pub mode: i32,
}

#[cfg(feature = "opencv")]
impl Default for StereoSgbmParams {
    fn default() -> Self {
        Self {
            min_disp: 0,
            num_disp: 16 * 8,
            block_size: 5,
            uniqueness_ratio: 10,
            speckle_window_size: 100,
            speckle_range: 32,
            disp12_max_diff: 1,
            mode: calib3d::StereoSGBM_MODE_SGBM_3WAY,
        }
    }
}

#[cfg(feature = "opencv")]
#[derive(Debug, Copy, Clone)]
pub struct DepthStats {
    pub disparity_ms: u128,
    pub reproject_ms: u128,
    pub points: usize,
}

#[cfg(feature = "opencv")]
pub fn depth_from_rectified_to_ply(
    left_path: &str,
    right_path: &str,
    calib_yaml: &str,
    out_depth_png: Option<&str>,
    out_ply: Option<&str>,
    roi: Option<(i32, i32, i32, i32)>,
    params: StereoSgbmParams,
) -> Result<DepthStats> {
    // Load calibration YAML
    let calib: CalibrationStereo = serde_yaml::from_str(
        &std::fs::read_to_string(calib_yaml).map_err(|e| Error::Io(e.to_string()))?,
    )
    .map_err(|e| Error::Io(e.to_string()))?;

    // Load images
    let img_l = imgcodecs::imread(left_path, imgcodecs::IMREAD_GRAYSCALE)
        .map_err(|e| Error::Backend(e.to_string()))?;
    let img_r = imgcodecs::imread(right_path, imgcodecs::IMREAD_GRAYSCALE)
        .map_err(|e| Error::Backend(e.to_string()))?;
    if img_l.empty() || img_r.empty() {
        return Err(Error::Io("failed to load input images".into()));
    }

    // Build SGBM matcher
    let block = params.block_size.max(3) | 1; // odd >=3
    let num_disp = if params.num_disp % 16 == 0 {
        params.num_disp
    } else {
        (params.num_disp / 16 + 1) * 16
    };
    let p1 = 8 * 1 * block * block; // grayscale
    let p2 = 32 * 1 * block * block;
    let sgbm = calib3d::StereoSGBM::create(
        params.min_disp,
        num_disp,
        block,
        p1,
        p2,
        params.disp12_max_diff,
        63,
        params.uniqueness_ratio,
        params.speckle_window_size,
        params.speckle_range,
        params.mode,
    )
    .map_err(|e| Error::Backend(e.to_string()))?;

    let mut disp = core::Mat::default();
    let t0 = std::time::Instant::now();
    sgbm.compute(&img_l, &img_r, &mut disp)
        .map_err(|e| Error::Backend(e.to_string()))?;
    let disparity_ms = t0.elapsed().as_millis();

    // Optional: write disparity visualization as 16-bit PNG
    if let Some(path) = out_depth_png {
        // Normalize disparity to 0-65535 for visualization
        let mut disp_u16 = core::Mat::default();
        let mut disp_f32 = core::Mat::default();
        disp.convert_to(&mut disp_f32, core::CV_32F, 1.0 / 16.0, 0.0)
            .map_err(|e| Error::Backend(e.to_string()))?;
        let mut disp_norm = core::Mat::default();
        core::normalize(
            &disp_f32,
            &mut disp_norm,
            0.0,
            65535.0,
            core::NORM_MINMAX,
            core::CV_32F,
            &core::no_array(),
        )
        .map_err(|e| Error::Backend(e.to_string()))?;
        disp_norm
            .convert_to(&mut disp_u16, core::CV_16U, 1.0, 0.0)
            .map_err(|e| Error::Backend(e.to_string()))?;
        imgcodecs::imwrite(path, &disp_u16, &core::Vector::new())
            .map_err(|e| Error::Backend(e.to_string()))?;
    }

    // Reproject to 3D using Q
    let q = mat_from_rows(&calib.q, 4, 4)?;
    let mut points = core::Mat::default();
    let mut disp_f32 = core::Mat::default();
    disp.convert_to(&mut disp_f32, core::CV_32F, 1.0 / 16.0, 0.0)
        .map_err(|e| Error::Backend(e.to_string()))?;
    let t1 = std::time::Instant::now();
    calib3d::reproject_image_to_3d(&disp_f32, &mut points, &q, false, core::CV_32F)
        .map_err(|e| Error::Backend(e.to_string()))?;
    let reproject_ms = t1.elapsed().as_millis();

    let points = count_and_maybe_write_ply(out_ply, &points, roi)?;
    Ok(DepthStats {
        disparity_ms,
        reproject_ms,
        points,
    })
}

#[cfg(feature = "opencv")]
fn mat_from_rows(rows: &Vec<Vec<f64>>, r: i32, c: i32) -> Result<core::Mat> {
    let mut m = core::Mat::zeros(r, c, core::CV_64F)
        .map_err(|e| Error::Backend(e.to_string()))?
        .to_mat()?;
    for i in 0..r as usize {
        for j in 0..c as usize {
            *m.at_2d_mut::<f64>(i as i32, j as i32)
                .map_err(|e| Error::Backend(e.to_string()))? = rows[i][j];
        }
    }
    Ok(m)
}

#[cfg(feature = "opencv")]
fn count_and_maybe_write_ply(
    path_opt: Option<&str>,
    points: &core::Mat,
    roi: Option<(i32, i32, i32, i32)>,
) -> Result<usize> {
    // points is HxWx3 CV_32F
    let rows = points.rows();
    let cols = points.cols();
    let (x0, y0, w, h) = roi.unwrap_or((0, 0, cols, rows));
    let x1 = (x0 + w).min(cols);
    let y1 = (y0 + h).min(rows);

    // Collect valid points (z finite)
    let mut buf = if path_opt.is_some() {
        String::new()
    } else {
        String::with_capacity(0)
    };
    let mut count = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = points
                .at_2d::<core::Vec3f>(y, x)
                .map_err(|e| Error::Backend(e.to_string()))?;
            let v = *p;
            let (X, Y, Z) = (v[0] as f64, v[1] as f64, v[2] as f64);
            if Z.is_finite() && Z < 1.0e4 && Z > -1.0e4 {
                if path_opt.is_some() {
                    buf.push_str(&format!("{X} {Y} {Z}\n"));
                }
                count += 1;
            }
        }
    }

    if let Some(path) = path_opt {
        // Write ASCII PLY
        let header = format!(
            "ply\nformat ascii 1.0\ncomment generated by vision-stereo\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nend_header\n",
            count
        );
        let mut out = String::with_capacity(header.len() + buf.len());
        out.push_str(&header);
        out.push_str(&buf);
        std::fs::write(path, out).map_err(|e| Error::Io(e.to_string()))?;
    }
    Ok(count)
}
