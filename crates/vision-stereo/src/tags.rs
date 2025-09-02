#[cfg(feature = "apriltag")]
use apriltag::{Detector, Family};
#[cfg(all(feature = "apriltag", feature = "opencv"))]
use opencv::{calib3d, core};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CameraIntrinsics {
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pose {
    pub r: [[f64; 3]; 3],
    pub t: [f64; 3],
}

#[cfg(all(feature = "apriltag", feature = "opencv"))]
pub fn estimate_tag_pose_from_image(
    image_gray: &[u8],
    width: usize,
    height: usize,
    intr: &CameraIntrinsics,
    tag_size_m: f64,
    dist_coeffs: Option<&[f64]>,
) -> crate::Result<Vec<Pose>> {
    let mut det =
        Detector::new(Family::tag36h11()).map_err(|e| crate::Error::Backend(e.to_string()))?;
    let results = det
        .detect(image_gray, width, height)
        .map_err(|e| crate::Error::Backend(e.to_string()))?;
    let mut out = Vec::new();
    for r in results {
        // 3D object points for an AprilTag centered at origin on Z=0 plane, size tag_size_m
        let s = tag_size_m / 2.0;
        let obj = vec![[-s, -s, 0.0], [s, -s, 0.0], [s, s, 0.0], [-s, s, 0.0]];
        // Image points from detection (counter-clockwise)
        let img = vec![
            [r.corners[0].x as f64, r.corners[0].y as f64],
            [r.corners[1].x as f64, r.corners[1].y as f64],
            [r.corners[2].x as f64, r.corners[2].y as f64],
            [r.corners[3].x as f64, r.corners[3].y as f64],
        ];

        // Build OpenCV Mats
        let mut obj_pts = core::Mat::zeros(obj.len() as i32, 1, core::CV_64FC3)
            .map_err(|e| crate::Error::Backend(e.to_string()))?;
        for (i, p) in obj.iter().enumerate() {
            let mut px = obj_pts
                .at_3d_mut::<f64>(i as i32)
                .map_err(|e| crate::Error::Backend(e.to_string()))?;
            px[0] = p[0];
            px[1] = p[1];
            px[2] = p[2];
        }
        let mut img_pts = core::Mat::zeros(img.len() as i32, 1, core::CV_64FC2)
            .map_err(|e| crate::Error::Backend(e.to_string()))?;
        for (i, p) in img.iter().enumerate() {
            let mut px = img_pts
                .at_2d_mut::<f64>(i as i32)
                .map_err(|e| crate::Error::Backend(e.to_string()))?;
            px[0] = p[0];
            px[1] = p[1];
        }
        let mut k = core::Mat::zeros(3, 3, core::CV_64F)
            .map_err(|e| crate::Error::Backend(e.to_string()))?;
        *k.at_2d_mut::<f64>(0, 0)
            .map_err(|e| crate::Error::Backend(e.to_string()))? = intr.fx;
        *k.at_2d_mut::<f64>(1, 1)
            .map_err(|e| crate::Error::Backend(e.to_string()))? = intr.fy;
        *k.at_2d_mut::<f64>(0, 2)
            .map_err(|e| crate::Error::Backend(e.to_string()))? = intr.cx;
        *k.at_2d_mut::<f64>(1, 2)
            .map_err(|e| crate::Error::Backend(e.to_string()))? = intr.cy;
        *k.at_2d_mut::<f64>(2, 2)
            .map_err(|e| crate::Error::Backend(e.to_string()))? = 1.0f64;
        // Build distortion coefficients mat (k1,k2,p1,p2,k3,...) if provided
        let mut dist = if let Some(d) = dist_coeffs {
            let n = d.len().max(4) as i32; // at least up to p2
            let mut m = core::Mat::zeros(n, 1, core::CV_64F)
                .map_err(|e| crate::Error::Backend(e.to_string()))?;
            for (i, v) in d.iter().enumerate() {
                *m.at::<f64>(i as i32)
                    .map_err(|e| crate::Error::Backend(e.to_string()))? = *v;
            }
            m
        } else {
            core::Mat::zeros(5, 1, core::CV_64F)
                .map_err(|e| crate::Error::Backend(e.to_string()))?
        };
        let mut rvec = core::Mat::default();
        let mut tvec = core::Mat::default();
        // Prefer IPPE for planar square if available, else fallback
        let method = calib3d::SOLVEPNP_IPPE_SQUARE;
        calib3d::solve_pnp(
            &obj_pts, &img_pts, &k, &dist, &mut rvec, &mut tvec, false, method,
        )
        .map_err(|e| crate::Error::Backend(e.to_string()))?;
        let mut rmat = core::Mat::default();
        calib3d::rodrigues(&rvec, &mut rmat, &mut core::no_array())
            .map_err(|e| crate::Error::Backend(e.to_string()))?;
        let mut r = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = *rmat
                    .at_2d::<f64>(i, j)
                    .map_err(|e| crate::Error::Backend(e.to_string()))?;
            }
        }
        let t = [
            *tvec
                .at::<f64>(0)
                .map_err(|e| crate::Error::Backend(e.to_string()))?,
            *tvec
                .at::<f64>(1)
                .map_err(|e| crate::Error::Backend(e.to_string()))?,
            *tvec
                .at::<f64>(2)
                .map_err(|e| crate::Error::Backend(e.to_string()))?,
        ];
        out.push(Pose { r, t });
    }
    Ok(out)
}
