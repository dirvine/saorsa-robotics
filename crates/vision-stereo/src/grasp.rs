use crate::{Error, Result};

/// Camera intrinsics used for back-projection
use crate::tags::CameraIntrinsics;

/// Minimal 6-DoF pose for grasps (rotation matrix + translation)
#[derive(Debug, Clone, Copy)]
pub struct GraspPose {
    pub r: [[f64; 3]; 3],
    pub t: [f64; 3],
}

impl GraspPose {
    pub fn identity() -> Self {
        Self {
            r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            t: [0.0, 0.0, 0.0],
        }
    }
}

/// Compute a surface normal at the center of an ROI from a depth map.
/// Depth is u16 millimeters; intrinsics are in pixels.
pub fn estimate_roi_normal(
    depth_mm: &[u16],
    width: usize,
    height: usize,
    intr: &CameraIntrinsics,
    roi: (i32, i32, i32, i32),
) -> Result<[f64; 3]> {
    if depth_mm.len() != width * height {
        return Err(Error::Backend("depth buffer size mismatch".to_string()));
    }
    let (x, y, w, h) = roi;
    if w <= 2 || h <= 2 {
        return Err(Error::Backend("ROI too small for normal estimation".to_string()));
    }
    let cx = (x + w / 2).max(1).min((width as i32) - 2) as usize;
    let cy = (y + h / 2).max(1).min((height as i32) - 2) as usize;

    // Sample a small cross around the center
    let pts = [
        (cx - 1, cy),
        (cx + 1, cy),
        (cx, cy - 1),
        (cx, cy + 1),
        (cx, cy),
    ];

    // Back-project helper
    let back_project = |u: usize, v: usize| -> Option<[f64; 3]> {
        let z_mm = depth_mm[v * width + u] as f64;
        if z_mm <= 0.0 {
            return None;
        }
        let z = z_mm * 1e-3; // meters
        let x = (u as f64 - intr.cx) / intr.fx * z;
        let y = (v as f64 - intr.cy) / intr.fy * z;
        Some([x, y, z])
    };

    let p_l = back_project(pts[0].0, pts[0].1).ok_or_else(|| Error::Backend("missing depth left".to_string()))?;
    let p_r = back_project(pts[1].0, pts[1].1).ok_or_else(|| Error::Backend("missing depth right".to_string()))?;
    let p_t = back_project(pts[2].0, pts[2].1).ok_or_else(|| Error::Backend("missing depth top".to_string()))?;
    let p_b = back_project(pts[3].0, pts[3].1).ok_or_else(|| Error::Backend("missing depth bottom".to_string()))?;

    // Tangent vectors
    let vx = [p_r[0] - p_l[0], p_r[1] - p_l[1], p_r[2] - p_l[2]];
    let vy = [p_b[0] - p_t[0], p_b[1] - p_t[1], p_b[2] - p_t[2]];
    // Normal via cross product
    let n = [
        vx[1] * vy[2] - vx[2] * vy[1],
        vx[2] * vy[0] - vx[0] * vy[2],
        vx[0] * vy[1] - vx[1] * vy[0],
    ];

    let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return Err(Error::Backend("degenerate normal".to_string()));
    }
    Ok([n[0] / norm, n[1] / norm, n[2] / norm])
}

/// Compute a simple grasp pose from an ROI using depth.
/// Returns pose in the camera frame; if camera_T_base is provided (row-major 4x4),
/// set `to_base=true` to transform the output into the base frame.
pub fn grasp_from_roi(
    depth_mm: &[u16],
    width: usize,
    height: usize,
    intr: &CameraIntrinsics,
    roi: (i32, i32, i32, i32),
    to_base: bool,
    camera_T_base: Option<[f32; 16]>,
) -> Result<GraspPose> {
    let n = estimate_roi_normal(depth_mm, width, height, intr, roi)?;

    // Center point in 3D
    let (x, y, w, h) = roi;
    let u = (x + w / 2).clamp(0, (width as i32) - 1) as usize;
    let v = (y + h / 2).clamp(0, (height as i32) - 1) as usize;
    let z_mm = depth_mm[v * width + u] as f64;
    if z_mm <= 0.0 {
        return Err(Error::Backend("missing center depth".to_string()));
    }
    let z = z_mm * 1e-3;
    let pc = [
        (u as f64 - intr.cx) / intr.fx * z,
        (v as f64 - intr.cy) / intr.fy * z,
        z,
    ];

    // Build a grasp frame: z-axis along -normal (approach), x-axis horizontal image direction
    let z_axis = [-n[0], -n[1], -n[2]];
    let x_hint = [1.0, 0.0, 0.0];
    let y_axis = cross(&z_axis, &x_hint);
    let y_norm = norm3(&y_axis);
    let y_axis = if y_norm > 0.0 { [y_axis[0] / y_norm, y_axis[1] / y_norm, y_axis[2] / y_norm] } else { [0.0, 1.0, 0.0] };
    let x_axis = cross(&y_axis, &z_axis);
    let x_norm = norm3(&x_axis);
    let x_axis = if x_norm > 0.0 { [x_axis[0] / x_norm, x_axis[1] / x_norm, x_axis[2] / x_norm] } else { [1.0, 0.0, 0.0] };

    let pose_cam = GraspPose {
        r: [x_axis, y_axis, z_axis],
        t: pc,
    };

    if to_base {
        if let Some(tcb) = camera_T_base {
            return Ok(transform_pose_row_major(&pose_cam, &tcb));
        }
        return Err(Error::Backend("camera_T_base required for base transform".to_string()));
    }
    Ok(pose_cam)
}

/// Derive a grasp pose from an AprilTag pose (camera frame) with an approach offset along tag normal.
pub fn grasp_from_tag(
    tag_r: [[f64; 3]; 3],
    tag_t: [f64; 3],
    approach_offset_m: f64,
    to_base: bool,
    camera_T_base: Option<[f32; 16]>,
) -> Result<GraspPose> {
    // Tag Z-axis assumed to be plane normal (pointing out of tag)
    let z_axis = [tag_r[0][2], tag_r[1][2], tag_r[2][2]];
    let x_axis = [tag_r[0][0], tag_r[1][0], tag_r[2][0]];
    let y_axis = [tag_r[0][1], tag_r[1][1], tag_r[2][1]];

    let t = [
        tag_t[0] - approach_offset_m * z_axis[0],
        tag_t[1] - approach_offset_m * z_axis[1],
        tag_t[2] - approach_offset_m * z_axis[2],
    ];

    let pose_cam = GraspPose { r: [x_axis, y_axis, z_axis], t };
    if to_base {
        if let Some(tcb) = camera_T_base {
            return Ok(transform_pose_row_major(&pose_cam, &tcb));
        }
        return Err(Error::Backend("camera_T_base required for base transform".to_string()));
    }
    Ok(pose_cam)
}

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(v: &[f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn transform_pose_row_major(p: &GraspPose, camera_T_base: &[f32; 16]) -> GraspPose {
    // camera_T_base is row-major 4x4; we want base frame pose: X_base = inv(camera_T_base) * X_cam
    // For simplicity in a demo, apply forward transform base_T_camera = inverse(camera_T_base) numerically
    // Compute inverse of 4x4 rigid transform (R t; 0 1): inv = (R^T, -R^T t)
    let r_cb = [
        [camera_T_base[0] as f64, camera_T_base[1] as f64, camera_T_base[2] as f64],
        [camera_T_base[4] as f64, camera_T_base[5] as f64, camera_T_base[6] as f64],
        [camera_T_base[8] as f64, camera_T_base[9] as f64, camera_T_base[10] as f64],
    ];
    let t_cb = [
        camera_T_base[3] as f64,
        camera_T_base[7] as f64,
        camera_T_base[11] as f64,
    ];
    // R_bc = R_cb^T
    let r_bc = [
        [r_cb[0][0], r_cb[1][0], r_cb[2][0]],
        [r_cb[0][1], r_cb[1][1], r_cb[2][1]],
        [r_cb[0][2], r_cb[1][2], r_cb[2][2]],
    ];
    let t_bc = [
        -(r_bc[0][0] * t_cb[0] + r_bc[0][1] * t_cb[1] + r_bc[0][2] * t_cb[2]),
        -(r_bc[1][0] * t_cb[0] + r_bc[1][1] * t_cb[1] + r_bc[1][2] * t_cb[2]),
        -(r_bc[2][0] * t_cb[0] + r_bc[2][1] * t_cb[1] + r_bc[2][2] * t_cb[2]),
    ];

    // Transform pose: R_b = R_bc * R_c, t_b = R_bc * t_c + t_bc
    let r_b = mat3_mul(&r_bc, &p.r);
    let t_b = [
        r_bc[0][0] * p.t[0] + r_bc[0][1] * p.t[1] + r_bc[0][2] * p.t[2] + t_bc[0],
        r_bc[1][0] * p.t[0] + r_bc[1][1] * p.t[1] + r_bc[1][2] * p.t[2] + t_bc[1],
        r_bc[2][0] * p.t[0] + r_bc[2][1] * p.t[1] + r_bc[2][2] * p.t[2] + t_bc[2],
    ];

    GraspPose { r: r_b, t: t_b }
}

fn mat3_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

