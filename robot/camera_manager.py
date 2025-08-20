#!/usr/bin/env python3
"""
Camera manager for SO-101 robotic arms with VLA model integration.
Supports USB cameras, Intel RealSense, and IP cameras.
Handles synchronized capture with 200Hz robot control loop.
"""

import cv2
import numpy as np
import asyncio
import threading
import queue
import time
import yaml
import json
import logging
from typing import Dict, Optional, Union, List, Tuple
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from collections import deque
from concurrent.futures import ThreadPoolExecutor

logger = logging.getLogger(__name__)


class CameraType(Enum):
    USB = "usb"
    REALSENSE = "realsense"
    IP = "ip"
    OPENCV = "opencv"


@dataclass
class CameraConfig:
    """Configuration for a single camera."""
    name: str
    type: CameraType
    device_id: Union[int, str]  # USB index, RealSense serial, or IP address
    width: int = 1280
    height: int = 720
    fps: int = 30
    buffer_size: int = 5
    mount_position: str = "wrist"  # wrist, table, overhead, side
    calibration_file: Optional[str] = None


class AsyncFrameBuffer:
    """Thread-safe frame buffer with timestamp synchronization."""
    
    def __init__(self, max_size: int = 10):
        self.buffer = deque(maxlen=max_size)
        self.lock = threading.Lock()
        self.new_frame_event = threading.Event()
        
    def put(self, frame: np.ndarray, timestamp: float):
        """Add frame with timestamp to buffer."""
        with self.lock:
            self.buffer.append((frame, timestamp))
            self.new_frame_event.set()
    
    def get_latest(self) -> Optional[Tuple[np.ndarray, float]]:
        """Get most recent frame without removing from buffer."""
        with self.lock:
            if self.buffer:
                return self.buffer[-1]
            return None
    
    def get_synchronized(self, target_timestamp: float, tolerance: float = 0.05) -> Optional[np.ndarray]:
        """Get frame closest to target timestamp within tolerance."""
        with self.lock:
            if not self.buffer:
                return None
            
            best_frame = None
            min_diff = float('inf')
            
            for frame, ts in self.buffer:
                diff = abs(ts - target_timestamp)
                if diff < min_diff and diff <= tolerance:
                    min_diff = diff
                    best_frame = frame
                    
            return best_frame
    
    def clear(self):
        """Clear all frames from buffer."""
        with self.lock:
            self.buffer.clear()
            self.new_frame_event.clear()


class CameraInterface:
    """Base interface for different camera types."""
    
    def __init__(self, config: CameraConfig):
        self.config = config
        self.is_running = False
        self.capture_thread = None
        self.frame_buffer = AsyncFrameBuffer(config.buffer_size)
        
    def start(self):
        """Start camera capture in background thread."""
        if self.is_running:
            return
            
        self.is_running = True
        self.capture_thread = threading.Thread(target=self._capture_loop, daemon=True)
        self.capture_thread.start()
        logger.info(f"Started camera {self.config.name} ({self.config.type.value})")
    
    def stop(self):
        """Stop camera capture."""
        self.is_running = False
        if self.capture_thread:
            self.capture_thread.join(timeout=2.0)
        logger.info(f"Stopped camera {self.config.name}")
    
    def _capture_loop(self):
        """Override in subclass for specific capture implementation."""
        raise NotImplementedError
    
    def get_frame(self) -> Optional[np.ndarray]:
        """Get latest frame."""
        result = self.frame_buffer.get_latest()
        return result[0] if result else None
    
    def get_frame_with_timestamp(self) -> Optional[Tuple[np.ndarray, float]]:
        """Get latest frame with timestamp."""
        return self.frame_buffer.get_latest()


class OpenCVCamera(CameraInterface):
    """OpenCV-based USB camera implementation."""
    
    def __init__(self, config: CameraConfig):
        super().__init__(config)
        self.cap = None
        
    def _capture_loop(self):
        """Capture frames from OpenCV VideoCapture."""
        self.cap = cv2.VideoCapture(int(self.config.device_id))
        
        # Set camera properties
        self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, self.config.width)
        self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, self.config.height)
        self.cap.set(cv2.CAP_PROP_FPS, self.config.fps)
        
        # Reduce buffer size to minimize latency
        self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
        
        while self.is_running:
            ret, frame = self.cap.read()
            if ret:
                timestamp = time.time()
                self.frame_buffer.put(frame, timestamp)
            else:
                logger.warning(f"Failed to read frame from {self.config.name}")
                time.sleep(0.001)
        
        self.cap.release()


class RealSenseCamera(CameraInterface):
    """Intel RealSense depth camera implementation."""
    
    def __init__(self, config: CameraConfig):
        super().__init__(config)
        self.pipeline = None
        self.align = None
        
    def _capture_loop(self):
        """Capture frames from RealSense camera."""
        try:
            import pyrealsense2 as rs
        except ImportError:
            logger.error("pyrealsense2 not installed. Install with: pip install pyrealsense2")
            return
        
        self.pipeline = rs.pipeline()
        config = rs.config()
        
        # Configure streams
        config.enable_stream(rs.stream.color, self.config.width, self.config.height, 
                           rs.format.bgr8, self.config.fps)
        config.enable_stream(rs.stream.depth, self.config.width, self.config.height,
                           rs.format.z16, self.config.fps)
        
        # Start pipeline
        profile = self.pipeline.start(config)
        
        # Align depth to color
        align_to = rs.stream.color
        self.align = rs.align(align_to)
        
        while self.is_running:
            frames = self.pipeline.wait_for_frames()
            aligned_frames = self.align.process(frames)
            
            color_frame = aligned_frames.get_color_frame()
            depth_frame = aligned_frames.get_depth_frame()
            
            if color_frame and depth_frame:
                timestamp = time.time()
                
                # Convert to numpy arrays
                color_image = np.asanyarray(color_frame.get_data())
                depth_image = np.asanyarray(depth_frame.get_data())
                
                # Stack RGB and depth (4 channels total)
                rgbd_image = np.dstack((color_image, depth_image[:,:,np.newaxis]))
                
                self.frame_buffer.put(rgbd_image, timestamp)
        
        self.pipeline.stop()


class IPCamera(CameraInterface):
    """IP/Network camera implementation."""
    
    def __init__(self, config: CameraConfig):
        super().__init__(config)
        self.stream_url = config.device_id  # Should be RTSP/HTTP URL
        self.cap = None
        
    def _capture_loop(self):
        """Capture frames from IP camera stream."""
        self.cap = cv2.VideoCapture(self.stream_url)
        
        # Set buffer size to reduce latency
        self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
        
        while self.is_running:
            ret, frame = self.cap.read()
            if ret:
                timestamp = time.time()
                self.frame_buffer.put(frame, timestamp)
            else:
                logger.warning(f"Failed to read frame from IP camera {self.config.name}")
                # Try to reconnect
                self.cap.release()
                time.sleep(1.0)
                self.cap = cv2.VideoCapture(self.stream_url)
        
        self.cap.release()


class CameraManager:
    """Manages multiple cameras for robot arm system."""
    
    def __init__(self, config_file: Optional[str] = None):
        self.cameras: Dict[str, CameraInterface] = {}
        self.recording_enabled = False
        self.recording_path = None
        self.video_writers: Dict[str, cv2.VideoWriter] = {}
        
        if config_file:
            self.load_config(config_file)
    
    def load_config(self, config_file: str):
        """Load camera configuration from YAML file."""
        with open(config_file, 'r') as f:
            config = yaml.safe_load(f)
        
        for cam_name, cam_config in config.get('cameras', {}).items():
            self.add_camera(
                name=cam_name,
                camera_type=cam_config.get('type', 'opencv'),
                device_id=cam_config.get('index_or_path', 0),
                width=cam_config.get('width', 1280),
                height=cam_config.get('height', 720),
                fps=cam_config.get('fps', 30),
                mount_position=cam_config.get('mount', 'wrist')
            )
    
    def add_camera(self, name: str, camera_type: str, device_id: Union[int, str],
                   width: int = 1280, height: int = 720, fps: int = 30,
                   mount_position: str = "wrist"):
        """Add a camera to the manager."""
        
        # Create camera configuration
        config = CameraConfig(
            name=name,
            type=CameraType(camera_type.lower()),
            device_id=device_id,
            width=width,
            height=height,
            fps=fps,
            mount_position=mount_position
        )
        
        # Create appropriate camera instance
        if config.type == CameraType.OPENCV or config.type == CameraType.USB:
            camera = OpenCVCamera(config)
        elif config.type == CameraType.REALSENSE:
            camera = RealSenseCamera(config)
        elif config.type == CameraType.IP:
            camera = IPCamera(config)
        else:
            raise ValueError(f"Unknown camera type: {config.type}")
        
        self.cameras[name] = camera
        logger.info(f"Added camera {name} ({camera_type}) at {mount_position}")
    
    def start_all(self):
        """Start all cameras."""
        for camera in self.cameras.values():
            camera.start()
    
    def stop_all(self):
        """Stop all cameras."""
        for camera in self.cameras.values():
            camera.stop()
        
        # Close video writers if recording
        if self.recording_enabled:
            self.stop_recording()
    
    def get_frames(self) -> Dict[str, np.ndarray]:
        """Get latest frames from all cameras."""
        frames = {}
        for name, camera in self.cameras.items():
            frame = camera.get_frame()
            if frame is not None:
                frames[name] = frame
        return frames
    
    def get_synchronized_frames(self, timestamp: float, tolerance: float = 0.05) -> Dict[str, np.ndarray]:
        """Get frames from all cameras synchronized to a specific timestamp."""
        frames = {}
        for name, camera in self.cameras.items():
            frame = camera.frame_buffer.get_synchronized(timestamp, tolerance)
            if frame is not None:
                frames[name] = frame
        return frames
    
    def start_recording(self, output_dir: str, episode_id: str):
        """Start recording video from all cameras."""
        self.recording_path = Path(output_dir) / episode_id
        self.recording_path.mkdir(parents=True, exist_ok=True)
        
        for name, camera in self.cameras.items():
            config = camera.config
            fourcc = cv2.VideoWriter_fourcc(*'mp4v')
            output_file = str(self.recording_path / f"{name}.mp4")
            
            writer = cv2.VideoWriter(
                output_file,
                fourcc,
                config.fps,
                (config.width, config.height)
            )
            
            self.video_writers[name] = writer
        
        self.recording_enabled = True
        logger.info(f"Started recording to {self.recording_path}")
    
    def stop_recording(self):
        """Stop recording video."""
        self.recording_enabled = False
        
        for writer in self.video_writers.values():
            writer.release()
        
        self.video_writers.clear()
        logger.info(f"Stopped recording")
    
    def save_frame_batch(self, frames: Dict[str, np.ndarray], timestamp: float):
        """Save a batch of frames with timestamp (for dataset creation)."""
        if self.recording_enabled:
            for name, frame in frames.items():
                if name in self.video_writers:
                    self.video_writers[name].write(frame)


class CameraCalibration:
    """Camera calibration utilities for SO-101 arms."""
    
    @staticmethod
    def calibrate_intrinsics(camera: CameraInterface, 
                            checkerboard_size: Tuple[int, int] = (9, 6),
                            num_frames: int = 30) -> Dict:
        """Calibrate camera intrinsics using checkerboard pattern."""
        criteria = (cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 30, 0.001)
        
        # Prepare object points
        objp = np.zeros((checkerboard_size[0] * checkerboard_size[1], 3), np.float32)
        objp[:, :2] = np.mgrid[0:checkerboard_size[0], 0:checkerboard_size[1]].T.reshape(-1, 2)
        
        objpoints = []
        imgpoints = []
        
        print(f"Calibrating {camera.config.name}. Show checkerboard pattern...")
        
        frames_captured = 0
        while frames_captured < num_frames:
            frame = camera.get_frame()
            if frame is None:
                continue
            
            gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
            ret, corners = cv2.findChessboardCorners(gray, checkerboard_size, None)
            
            if ret:
                objpoints.append(objp)
                corners2 = cv2.cornerSubPix(gray, corners, (11, 11), (-1, -1), criteria)
                imgpoints.append(corners2)
                frames_captured += 1
                
                # Draw and display
                cv2.drawChessboardCorners(frame, checkerboard_size, corners2, ret)
                cv2.putText(frame, f"Captured: {frames_captured}/{num_frames}", 
                          (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 1, (0, 255, 0), 2)
            
            cv2.imshow(f"Calibration - {camera.config.name}", frame)
            if cv2.waitKey(100) & 0xFF == ord('q'):
                break
        
        cv2.destroyAllWindows()
        
        # Calibrate
        ret, mtx, dist, rvecs, tvecs = cv2.calibrateCamera(
            objpoints, imgpoints, gray.shape[::-1], None, None
        )
        
        return {
            'camera_matrix': mtx.tolist(),
            'distortion_coeffs': dist.tolist(),
            'calibration_error': ret
        }
    
    @staticmethod
    def calibrate_extrinsics(camera1: CameraInterface, camera2: CameraInterface,
                            checkerboard_size: Tuple[int, int] = (9, 6)) -> Dict:
        """Calibrate extrinsics between two cameras."""
        # Implementation for stereo calibration
        pass
    
    @staticmethod
    def save_calibration(calibration_data: Dict, output_file: str):
        """Save calibration data to YAML file."""
        with open(output_file, 'w') as f:
            yaml.dump(calibration_data, f)
    
    @staticmethod
    def load_calibration(calibration_file: str) -> Dict:
        """Load calibration data from YAML file."""
        with open(calibration_file, 'r') as f:
            return yaml.safe_load(f)


# Example usage for async integration with robot control
async def camera_robot_sync_example():
    """Example of synchronizing camera capture with robot control loop."""
    
    # Initialize camera manager
    cam_manager = CameraManager()
    cam_manager.add_camera("wrist", "opencv", 0)
    cam_manager.add_camera("table", "opencv", 1)
    cam_manager.start_all()
    
    # Simulated robot control loop at 200Hz
    control_rate = 200  # Hz
    dt = 1.0 / control_rate
    
    try:
        while True:
            loop_start = time.time()
            
            # Get current timestamp for synchronization
            current_time = time.time()
            
            # Get synchronized frames (will use closest frames within tolerance)
            frames = cam_manager.get_synchronized_frames(current_time, tolerance=0.02)
            
            # Process frames for VLA model (async)
            # This would be sent to remote GPU server
            if frames:
                # Prepare observation for policy
                observation = {
                    'images': frames,
                    'timestamp': current_time
                }
                # Send to policy server asynchronously
                # action = await policy_client.get_action(observation)
            
            # Sleep to maintain control rate
            elapsed = time.time() - loop_start
            if elapsed < dt:
                await asyncio.sleep(dt - elapsed)
            
    finally:
        cam_manager.stop_all()


if __name__ == "__main__":
    # Test camera manager
    logging.basicConfig(level=logging.INFO)
    
    manager = CameraManager()
    manager.add_camera("test_cam", "opencv", 0)
    manager.start_all()
    
    try:
        for i in range(100):
            frames = manager.get_frames()
            if "test_cam" in frames:
                cv2.imshow("Test", frames["test_cam"])
                if cv2.waitKey(1) & 0xFF == ord('q'):
                    break
            time.sleep(0.033)  # ~30 FPS
    finally:
        manager.stop_all()
        cv2.destroyAllWindows()