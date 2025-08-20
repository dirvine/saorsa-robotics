#!/usr/bin/env python3
"""
Camera calibration utility for SO-101 robotic arms.
Supports multiple camera types and saves calibration data.
"""

import cv2
import numpy as np
import yaml
import argparse
from pathlib import Path
import time

class CameraCalibrator:
    def __init__(self, camera_id=0, calibration_pattern=(9, 6)):
        self.camera_id = camera_id
        self.pattern_size = calibration_pattern
        self.square_size = 0.025  # 25mm squares
        
    def calibrate_intrinsics(self, num_images=20):
        """Calibrate camera intrinsics using checkerboard pattern."""
        cap = cv2.VideoCapture(self.camera_id)
        
        # Prepare object points
        objp = np.zeros((self.pattern_size[0] * self.pattern_size[1], 3), np.float32)
        objp[:, :2] = np.mgrid[0:self.pattern_size[0], 0:self.pattern_size[1]].T.reshape(-1, 2)
        objp *= self.square_size
        
        objpoints = []  # 3D points in real world
        imgpoints = []  # 2D points in image plane
        
        print(f"Starting calibration. Press SPACE to capture, ESC to finish.")
        images_captured = 0
        
        while images_captured < num_images:
            ret, frame = cap.read()
            if not ret:
                continue
                
            gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
            ret, corners = cv2.findChessboardCorners(gray, self.pattern_size, None)
            
            if ret:
                cv2.drawChessboardCorners(frame, self.pattern_size, corners, ret)
                
            cv2.putText(frame, f"Images: {images_captured}/{num_images}", 
                       (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 1, (0, 255, 0), 2)
            cv2.imshow('Calibration', frame)
            
            key = cv2.waitKey(1)
            if key == ord(' ') and ret:
                objpoints.append(objp)
                imgpoints.append(corners)
                images_captured += 1
                print(f"Captured image {images_captured}/{num_images}")
            elif key == 27:  # ESC
                break
                
        cap.release()
        cv2.destroyAllWindows()
        
        if len(objpoints) > 0:
            ret, mtx, dist, rvecs, tvecs = cv2.calibrateCamera(
                objpoints, imgpoints, gray.shape[::-1], None, None
            )
            
            return {
                'camera_matrix': mtx.tolist(),
                'distortion_coeffs': dist.tolist(),
                'calibration_error': ret
            }
        return None
        
    def save_calibration(self, calibration_data, filepath):
        """Save calibration data to YAML file."""
        with open(filepath, 'w') as f:
            yaml.dump(calibration_data, f)
        print(f"Calibration saved to {filepath}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Calibrate cameras for SO-101 arms')
    parser.add_argument('--camera', type=int, default=0, help='Camera ID')
    parser.add_argument('--output', type=str, default='robot/configs/camera_calib.yaml')
    args = parser.parse_args()
    
    calibrator = CameraCalibrator(args.camera)
    calib_data = calibrator.calibrate_intrinsics()
    
    if calib_data:
        calibrator.save_calibration(calib_data, args.output)
