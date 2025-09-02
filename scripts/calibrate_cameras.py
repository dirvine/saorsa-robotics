#!/usr/bin/env python3
"""
Camera calibration utility for SO-101 robotic arms.

Supports multiple camera types and saves calibration data.
"""

import cv2
import numpy as np
import numpy.typing as npt
import yaml
import argparse
import logging
from pathlib import Path
from typing import Optional, Tuple, List, Dict, Any
import sys

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class CalibrationError(Exception):
    """Custom exception for calibration errors."""
    pass


class CameraCalibrator:
    """Camera calibration utility for intrinsic and extrinsic parameters."""
    
    def __init__(
        self, 
        camera_id: int = 0, 
        calibration_pattern: Tuple[int, int] = (9, 6),
        square_size: float = 0.025
    ) -> None:
        """
        Initialize camera calibrator.
        
        Args:
            camera_id: Camera device ID (USB index)
            calibration_pattern: Checkerboard inner corners (width, height)
            square_size: Size of checkerboard squares in meters
        """
        if camera_id < 0:
            raise ValueError(f"Camera ID must be non-negative, got {camera_id}")
        if calibration_pattern[0] <= 0 or calibration_pattern[1] <= 0:
            raise ValueError(f"Invalid calibration pattern: {calibration_pattern}")
        if square_size <= 0:
            raise ValueError(f"Square size must be positive, got {square_size}")
            
        self.camera_id = camera_id
        self.pattern_size = calibration_pattern
        self.square_size = square_size  # Size in meters
        self.cap: Optional[cv2.VideoCapture] = None
        
    def _validate_camera_connection(self) -> None:
        """Validate that camera can be opened."""
        test_cap = cv2.VideoCapture(self.camera_id)
        if not test_cap.isOpened():
            raise CalibrationError(f"Cannot open camera {self.camera_id}")
        test_cap.release()
        
    def calibrate_intrinsics(self, num_images: int = 20) -> Optional[Dict[str, Any]]:
        """
        Calibrate camera intrinsics using checkerboard pattern.
        
        Args:
            num_images: Number of calibration images to capture
            
        Returns:
            Dictionary containing calibration parameters or None if cancelled
        """
        if num_images <= 0:
            raise ValueError(f"Number of images must be positive, got {num_images}")
            
        # Validate camera before starting
        self._validate_camera_connection()
        
        try:
            self.cap = cv2.VideoCapture(self.camera_id)
            
            if not self.cap.isOpened():
                raise CalibrationError(f"Failed to open camera {self.camera_id}")
            
            # Set camera properties for better calibration
            self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1280)
            self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)
            self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
            
            # Prepare object points
            objp = np.zeros((self.pattern_size[0] * self.pattern_size[1], 3), np.float32)
            objp[:, :2] = np.mgrid[0:self.pattern_size[0], 0:self.pattern_size[1]].T.reshape(-1, 2)
            objp *= self.square_size
            
            objpoints: List[npt.NDArray[np.float32]] = []  # 3D points in real world
            imgpoints: List[npt.NDArray[np.float32]] = []  # 2D points in image plane
            
            logger.info("Starting calibration. Press SPACE to capture, ESC to finish.")
            images_captured = 0
            criteria = (cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 30, 0.001)
            
            frame_shape = None
            consecutive_failures = 0
            max_consecutive_failures = 30
            
            while images_captured < num_images:
                ret, frame = self.cap.read()
                if not ret or frame is None:
                    consecutive_failures += 1
                    if consecutive_failures > max_consecutive_failures:
                        logger.error("Too many consecutive read failures")
                        break
                    continue
                    
                consecutive_failures = 0
                frame_shape = frame.shape[:2]
                    
                gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
                ret, corners = cv2.findChessboardCorners(gray, self.pattern_size, None)
                
                display_frame = frame.copy()
                
                if ret:
                    # Refine corner positions
                    corners_refined = cv2.cornerSubPix(
                        gray, corners, (11, 11), (-1, -1), criteria
                    )
                    cv2.drawChessboardCorners(display_frame, self.pattern_size, corners_refined, ret)
                    status_color = (0, 255, 0)  # Green when pattern found
                else:
                    status_color = (0, 0, 255)  # Red when pattern not found
                    
                # Display status
                cv2.putText(
                    display_frame, 
                    f"Images: {images_captured}/{num_images} - {'Pattern found' if ret else 'No pattern'}", 
                    (10, 30), 
                    cv2.FONT_HERSHEY_SIMPLEX, 
                    0.7, 
                    status_color, 
                    2
                )
                cv2.putText(
                    display_frame,
                    "Press SPACE to capture, ESC to finish",
                    (10, 60),
                    cv2.FONT_HERSHEY_SIMPLEX,
                    0.5,
                    (255, 255, 255),
                    1
                )
                
                cv2.imshow('Camera Calibration', display_frame)
                
                key = cv2.waitKey(1) & 0xFF
                if key == ord(' ') and ret:
                    objpoints.append(objp)
                    imgpoints.append(corners_refined)
                    images_captured += 1
                    logger.info(f"Captured calibration image {images_captured}/{num_images}")
                elif key == 27:  # ESC
                    logger.info("Calibration cancelled by user")
                    break
                    
        except Exception as e:
            logger.error(f"Error during calibration: {e}")
            raise
        finally:
            if self.cap:
                self.cap.release()
            cv2.destroyAllWindows()
        
        if len(objpoints) < 5:
            logger.warning(f"Insufficient calibration images: {len(objpoints)}. Need at least 5.")
            return None
            
        if frame_shape is None:
            logger.error("No frames captured")
            return None
            
        logger.info(f"Calibrating with {len(objpoints)} images...")
        
        try:
            ret, mtx, dist, rvecs, tvecs = cv2.calibrateCamera(
                objpoints, imgpoints, frame_shape[::-1], None, None
            )
            
            # Calculate reprojection error
            total_error = 0.0
            for i in range(len(objpoints)):
                imgpoints2, _ = cv2.projectPoints(objpoints[i], rvecs[i], tvecs[i], mtx, dist)
                error = cv2.norm(imgpoints[i], imgpoints2, cv2.NORM_L2) / len(imgpoints2)
                total_error += error
                
            mean_error = total_error / len(objpoints)
            
            calibration_data = {
                'camera_id': self.camera_id,
                'camera_matrix': mtx.tolist(),
                'distortion_coeffs': dist.tolist(),
                'calibration_error': float(ret),
                'reprojection_error': float(mean_error),
                'num_images_used': len(objpoints),
                'image_size': list(frame_shape[::-1]),
                'pattern_size': list(self.pattern_size),
                'square_size': self.square_size
            }
            
            logger.info(f"Calibration complete. RMS error: {ret:.3f}, Reprojection error: {mean_error:.3f}")
            
            return calibration_data
            
        except Exception as e:
            logger.error(f"Calibration computation failed: {e}")
            return None
        
    def save_calibration(self, calibration_data: Dict[str, Any], filepath: str) -> None:
        """
        Save calibration data to YAML file.
        
        Args:
            calibration_data: Calibration parameters to save
            filepath: Path to output YAML file
        """
        if not calibration_data:
            raise ValueError("No calibration data to save")
            
        output_path = Path(filepath)
        
        # Create parent directory if it doesn't exist
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Validate path doesn't contain traversal attempts
        try:
            output_path.resolve().relative_to(Path.cwd())
        except ValueError:
            if not output_path.is_absolute():
                raise ValueError(f"Invalid output path: {filepath}")
        
        try:
            with open(output_path, 'w') as f:
                yaml.dump(calibration_data, f, default_flow_style=False, sort_keys=False)
            logger.info(f"Calibration saved to {output_path}")
        except IOError as e:
            raise CalibrationError(f"Failed to save calibration: {e}") from e
            
    def load_calibration(self, filepath: str) -> Dict[str, Any]:
        """
        Load calibration data from YAML file.
        
        Args:
            filepath: Path to calibration YAML file
            
        Returns:
            Dictionary containing calibration parameters
        """
        calib_path = Path(filepath)
        
        if not calib_path.exists():
            raise FileNotFoundError(f"Calibration file not found: {calib_path}")
            
        try:
            with open(calib_path, 'r') as f:
                calibration_data = yaml.safe_load(f)
                
            if not isinstance(calibration_data, dict):
                raise ValueError("Invalid calibration file format")
                
            # Validate required fields
            required_fields = ['camera_matrix', 'distortion_coeffs']
            for field in required_fields:
                if field not in calibration_data:
                    raise ValueError(f"Missing required field: {field}")
                    
            return calibration_data
            
        except yaml.YAMLError as e:
            raise CalibrationError(f"Failed to parse calibration file: {e}") from e
            
    def test_calibration(self, calibration_file: str) -> None:
        """
        Test calibration by showing undistorted live feed.
        
        Args:
            calibration_file: Path to calibration YAML file
        """
        calib_data = self.load_calibration(calibration_file)
        
        mtx = np.array(calib_data['camera_matrix'])
        dist = np.array(calib_data['distortion_coeffs'])
        
        self.cap = cv2.VideoCapture(self.camera_id)
        
        if not self.cap.isOpened():
            raise CalibrationError(f"Failed to open camera {self.camera_id}")
            
        logger.info("Showing undistorted feed. Press 'q' to quit.")
        
        try:
            while True:
                ret, frame = self.cap.read()
                if not ret or frame is None:
                    continue
                    
                h, w = frame.shape[:2]
                newcameramtx, roi = cv2.getOptimalNewCameraMatrix(
                    mtx, dist, (w, h), 1, (w, h)
                )
                
                # Undistort
                undistorted = cv2.undistort(frame, mtx, dist, None, newcameramtx)
                
                # Crop the image
                x, y, w, h = roi
                if w > 0 and h > 0:
                    undistorted = undistorted[y:y+h, x:x+w]
                
                # Show both original and undistorted
                display = np.hstack([
                    cv2.resize(frame, (640, 480)),
                    cv2.resize(undistorted, (640, 480))
                ])
                
                cv2.putText(
                    display,
                    "Original",
                    (10, 30),
                    cv2.FONT_HERSHEY_SIMPLEX,
                    1,
                    (0, 255, 0),
                    2
                )
                cv2.putText(
                    display,
                    "Undistorted",
                    (650, 30),
                    cv2.FONT_HERSHEY_SIMPLEX,
                    1,
                    (0, 255, 0),
                    2
                )
                
                cv2.imshow('Calibration Test', display)
                
                if cv2.waitKey(1) & 0xFF == ord('q'):
                    break
                    
        finally:
            self.cap.release()
            cv2.destroyAllWindows()


def main() -> None:
    """Main function for camera calibration utility."""
    parser = argparse.ArgumentParser(
        description='Calibrate cameras for SO-101 robotic arms',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument(
        '--camera', 
        type=int, 
        default=0, 
        help='Camera device ID'
    )
    parser.add_argument(
        '--output', 
        type=str, 
        default='robot/configs/camera_calib.yaml',
        help='Output calibration file path'
    )
    parser.add_argument(
        '--pattern', 
        type=str,
        default='9x6',
        help='Checkerboard pattern size (e.g., 9x6)'
    )
    parser.add_argument(
        '--square-size',
        type=float,
        default=0.025,
        help='Checkerboard square size in meters'
    )
    parser.add_argument(
        '--num-images',
        type=int,
        default=20,
        help='Number of calibration images to capture'
    )
    parser.add_argument(
        '--test',
        type=str,
        help='Test existing calibration file'
    )
    
    args = parser.parse_args()
    
    # Parse pattern size
    try:
        pattern_parts = args.pattern.split('x')
        if len(pattern_parts) != 2:
            raise ValueError("Pattern must be in format WxH (e.g., 9x6)")
        pattern_size = (int(pattern_parts[0]), int(pattern_parts[1]))
    except (ValueError, IndexError):
        logger.error(f"Invalid pattern format: {args.pattern}")
        sys.exit(1)
    
    try:
        calibrator = CameraCalibrator(
            camera_id=args.camera,
            calibration_pattern=pattern_size,
            square_size=args.square_size
        )
        
        if args.test:
            # Test mode - show undistorted feed
            calibrator.test_calibration(args.test)
        else:
            # Calibration mode
            calib_data = calibrator.calibrate_intrinsics(num_images=args.num_images)
            
            if calib_data:
                calibrator.save_calibration(calib_data, args.output)
                logger.info("Calibration successful!")
            else:
                logger.warning("Calibration failed or was cancelled")
                sys.exit(1)
                
    except Exception as e:
        logger.error(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()