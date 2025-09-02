use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[cfg(feature = "opencv")]
use opencv::{calib3d, core, imgcodecs, imgproc, prelude::*, types};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationStereo {
    pub image_width: i32,
    pub image_height: i32,
    pub grid_rows: i32,
    pub grid_cols: i32,
    pub square_mm: f64,

    pub k1: Vec<Vec<f64>>,
    pub d1: Vec<f64>,
    pub k2: Vec<Vec<f64>>,
    pub d2: Vec<f64>,

    pub r: Vec<Vec<f64>>,
    pub t: Vec<f64>,

    pub r1: Vec<Vec<f64>>,
    pub r2: Vec<Vec<f64>>,
    pub p1: Vec<Vec<f64>>,
    pub p2: Vec<Vec<f64>>,
    pub q: Vec<Vec<f64>>,
}

#[cfg(feature = "opencv")]
pub fn stereo_calibrate(
    left_dir: &str,
    right_dir: &str,
    rows: i32,
    cols: i32,
    square_mm: f64,
) -> Result<CalibrationStereo> {
    // Collect paired images (sorted by filename)
    fn list_sorted(dir: &str) -> Result<Vec<String>> {
        let mut v = Vec::new();
        for e in fs::read_dir(dir).map_err(|e| Error::Io(e.to_string()))? {
            let p = e.map_err(|e| Error::Io(e.to_string()))?.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                let ex = ext.to_ascii_lowercase();
                if ["png", "jpg", "jpeg", "bmp"].contains(&ex.as_str()) {
                    v.push(p.to_string_lossy().to_string());
                }
            }
        }
        v.sort();
        Ok(v)
    }

    let left = list_sorted(left_dir)?;
    let right = list_sorted(right_dir)?;
    let n = left.len().min(right.len());
    if n < 5 {
        return Err(Error::Io("need at least 5 image pairs".into()));
    }

    let pattern = core::Size::new(cols, rows);
    let term = core::TermCriteria::new(
        core::TermCriteria_Type::COUNT | core::TermCriteria_Type::EPS,
        30,
        0.01,
    )?;

    let mut object_points = types::VectorOfMat::new();
    let mut image_points_l = types::VectorOfMat::new();
    let mut image_points_r = types::VectorOfMat::new();
    let mut image_size = core::Size::new(0, 0);

    // Prepare single object grid
    let mut obj = core::Mat::zeros(rows * cols, 1, core::CV_32FC3)?.to_mat()?;
    for r in 0..rows {
        for c in 0..cols {
            let idx = (r * cols + c) as i32;
            let mut px = obj.at_3d_mut::<f32>(idx)?;
            px[0] = c as f32 * (square_mm as f32);
            px[1] = r as f32 * (square_mm as f32);
            px[2] = 0.0;
        }
    }

    for i in 0..n {
        let il = imgcodecs::imread(&left[i], imgcodecs::IMREAD_GRAYSCALE)?;
        let ir = imgcodecs::imread(&right[i], imgcodecs::IMREAD_GRAYSCALE)?;
        if il.empty() || ir.empty() {
            continue;
        }
        image_size = il.size()?;

        let mut corners_l = core::Mat::default();
        let mut corners_r = core::Mat::default();
        let found_l = calib3d::find_chessboard_corners(
            &il,
            pattern,
            &mut corners_l,
            calib3d::CALIB_CB_ADAPTIVE_THRESH | calib3d::CALIB_CB_NORMALIZE_IMAGE,
        )?;
        let found_r = calib3d::find_chessboard_corners(
            &ir,
            pattern,
            &mut corners_r,
            calib3d::CALIB_CB_ADAPTIVE_THRESH | calib3d::CALIB_CB_NORMALIZE_IMAGE,
        )?;
        if !(found_l && found_r) {
            continue;
        }

        imgproc::corner_sub_pix(
            &il,
            &mut corners_l,
            core::Size::new(11, 11),
            core::Size::new(-1, -1),
            term,
        )?;
        imgproc::corner_sub_pix(
            &ir,
            &mut corners_r,
            core::Size::new(11, 11),
            core::Size::new(-1, -1),
            term,
        )?;

        object_points.push(obj.clone());
        image_points_l.push(corners_l);
        image_points_r.push(corners_r);
    }

    if object_points.len() < 5 {
        return Err(Error::Io(
            "insufficient valid pairs with detected corners".into(),
        ));
    }

    // Stereo calibrate
    let mut k1 = core::Mat::eye(3, 3, core::CV_64F)?.to_mat()?;
    let mut d1 = core::Mat::zeros(5, 1, core::CV_64F)?.to_mat()?;
    let mut k2 = core::Mat::eye(3, 3, core::CV_64F)?.to_mat()?;
    let mut d2 = core::Mat::zeros(5, 1, core::CV_64F)?.to_mat()?;
    let mut r = core::Mat::default();
    let mut t = core::Mat::default();
    let mut e = core::Mat::default();
    let mut f = core::Mat::default();
    let criteria = core::TermCriteria::new(
        core::TermCriteria_Type::COUNT | core::TermCriteria_Type::EPS,
        100,
        1e-6,
    )?;
    let flags = 0;
    let _rms = calib3d::stereo_calibrate(
        &object_points,
        &image_points_l,
        &image_points_r,
        &mut k1,
        &mut d1,
        &mut k2,
        &mut d2,
        image_size,
        &mut r,
        &mut t,
        &mut e,
        &mut f,
        flags,
        criteria,
    )?;

    // Rectification
    let mut r1 = core::Mat::default();
    let mut r2 = core::Mat::default();
    let mut p1 = core::Mat::default();
    let mut p2 = core::Mat::default();
    let mut q = core::Mat::default();
    calib3d::stereo_rectify(
        &k1,
        &d1,
        &k2,
        &d2,
        image_size,
        &r,
        &t,
        &mut r1,
        &mut r2,
        &mut p1,
        &mut p2,
        &mut q,
        calib3d::CALIB_ZERO_DISPARITY as i32,
        0.0,
        image_size,
        &mut core::no_array(),
        &mut core::no_array(),
    )?;

    Ok(CalibrationStereo {
        image_width: image_size.width,
        image_height: image_size.height,
        grid_rows: rows,
        grid_cols: cols,
        square_mm,
        k1: mat_to_rows(&k1, 3, 3)?,
        d1: mat_to_vec(&d1)?,
        k2: mat_to_rows(&k2, 3, 3)?,
        d2: mat_to_vec(&d2)?,
        r: mat_to_rows(&r, 3, 3)?,
        t: mat_to_vec(&t)?,
        r1: mat_to_rows(&r1, 3, 3)?,
        r2: mat_to_rows(&r2, 3, 3)?,
        p1: mat_to_rows(&p1, 3, 4)?,
        p2: mat_to_rows(&p2, 3, 4)?,
        q: mat_to_rows(&q, 4, 4)?,
    })
}

#[cfg(feature = "opencv")]
fn mat_to_rows(m: &core::Mat, rows: i32, cols: i32) -> Result<Vec<Vec<f64>>> {
    let mut out = vec![vec![0.0f64; cols as usize]; rows as usize];
    for r in 0..rows {
        for c in 0..cols {
            out[r as usize][c as usize] = *m
                .at_2d::<f64>(r, c)
                .map_err(|e| Error::Backend(e.to_string()))?;
        }
    }
    Ok(out)
}

#[cfg(feature = "opencv")]
fn mat_to_vec(m: &core::Mat) -> Result<Vec<f64>> {
    let total = m.total()? as usize;
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        out.push(
            *m.at::<f64>(i as i32)
                .map_err(|e| Error::Backend(e.to_string()))?,
        );
    }
    Ok(out)
}

pub fn write_yaml(calib: &CalibrationStereo, path: &str) -> Result<()> {
    let s = serde_yaml::to_string(calib).map_err(|e| Error::Io(e.to_string()))?;
    fs::write(Path::new(path), s).map_err(|e| Error::Io(e.to_string()))
}
