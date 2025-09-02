#!/usr/bin/env python3
"""
Camera manager for SO-101 robotic arms with VLA model integration.

Supports USB cameras, Intel RealSense, and IP cameras.
Handles synchronized capture with 200Hz robot control loop.
"""

import cv2
import numpy as np
import numpy.typing as npt
import asyncio
import threading
import time
import yaml
import logging
from typing import Dict, Optional, Union, List, Tuple, Any
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from collections import deque

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class CameraError(Exception):
    """Custom exception for camera-related errors."""
    pass


class CameraType(Enum):
    """Supported camera types."""
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
    max_reconnect_attempts: int = 3
    reconnect_delay: float = 1.0


class AsyncFrameBuffer:
    """Thread-safe frame buffer with timestamp synchronization."""
    
    def __init__(self, max_size: int = 10) -> None:
        """
        Initialize frame buffer.
        
        Args:
            max_size: Maximum number of frames to buffer
        """
        if max_size <= 0:
            raise ValueError(f"max_size must be positive, got {max_size}")
            
        self.buffer: deque[Tuple[npt.NDArray[np.uint8], float]] = deque(maxlen=max_size)
        self.lock = threading.RLock()
        self.new_frame_event = threading.Event()
        
    def put(self, frame: npt.NDArray[np.uint8], timestamp: float) -> None:
        """
        Add frame with timestamp to buffer.
        
        Args:
            frame: Image frame as numpy array
            timestamp: Unix timestamp when frame was captured
        """
        if frame is None:
            raise ValueError("Frame cannot be None")
        if timestamp <= 0:
            raise ValueError(f"Invalid timestamp: {timestamp}")
            
        with self.lock:
            self.buffer.append((frame, timestamp))
            self.new_frame_event.set()
    
    def get_latest(self) -> Optional[Tuple[npt.NDArray[np.uint8], float]]:
        """
        Get most recent frame without removing from buffer.
        
        Returns:
            Tuple of (frame, timestamp) or None if buffer is empty
        """
        with self.lock:
            if self.buffer:
                return self.buffer[-1]
            return None
    
    def get_synchronized(
        self, 
        target_timestamp: float, 
        tolerance: float = 0.05
    ) -> Optional[npt.NDArray[np.uint8]]:
        """
        Get frame closest to target timestamp within tolerance.
        
        Args:
            target_timestamp: Target timestamp to synchronize to
            tolerance: Maximum time difference allowed
            
        Returns:
            Frame closest to target timestamp or None if no match
        """
        if tolerance <= 0:
            raise ValueError(f"Tolerance must be positive, got {tolerance}")
            
        with self.lock:
            if not self.buffer:
                return None
            
            best_frame: Optional[npt.NDArray[np.uint8]] = None
            min_diff = float('inf')
            
            for frame, ts in self.buffer:
                diff = abs(ts - target_timestamp)
                if diff < min_diff and diff <= tolerance:
                    min_diff = diff
                    best_frame = frame
                    
            return best_frame
    
    def clear(self) -> None:
        """Clear all frames from buffer."""
        with self.lock:
            self.buffer.clear()
            self.new_frame_event.clear()


class CameraInterface:
    """Base interface for different camera types."""
    
    def __init__(self, config: CameraConfig) -> None:
        """
        Initialize camera interface.
        
        Args:
            config: Camera configuration
        """
        self.config = config
        self.is_running = False
        self.capture_thread: Optional[threading.Thread] = None
        self.frame_buffer = AsyncFrameBuffer(config.buffer_size)
        self._lock = threading.RLock()
        
    def start(self) -> None:
        """Start camera capture in background thread."""
        with self._lock:
            if self.is_running:
                logger.warning(f"Camera {self.config.name} already running")
                return
                
            self.is_running = True
            self.capture_thread = threading.Thread(
                target=self._capture_loop_wrapper, 
                daemon=True,
                name=f"Camera-{self.config.name}"
            )
            self.capture_thread.start()
            logger.info(f"Started camera {self.config.name} ({self.config.type.value})")
    
    def stop(self) -> None:
        """Stop camera capture."""
        with self._lock:
            if not self.is_running:
                return
                
            self.is_running = False
            
        if self.capture_thread:
            self.capture_thread.join(timeout=2.0)
            if self.capture_thread.is_alive():
                logger.warning(f"Camera {self.config.name} thread did not stop cleanly")
                
        logger.info(f"Stopped camera {self.config.name}")
    
    def _capture_loop_wrapper(self) -> None:
        """Wrapper for capture loop with error handling."""
        try:
            self._capture_loop()
        except Exception as e:
            logger.error(f"Camera {self.config.name} capture loop failed: {e}")
            self.is_running = False
    
    def _capture_loop(self) -> None:
        """Override in subclass for specific capture implementation."""
        raise NotImplementedError
    
    def get_frame(self) -> Optional[npt.NDArray[np.uint8]]:
        """
        Get latest frame.
        
        Returns:
            Latest frame or None if no frame available
        """
        result = self.frame_buffer.get_latest()
        return result[0] if result else None
    
    def get_frame_with_timestamp(self) -> Optional[Tuple[npt.NDArray[np.uint8], float]]:
        """
        Get latest frame with timestamp.
        
        Returns:
            Tuple of (frame, timestamp) or None if no frame available
        """
        return self.frame_buffer.get_latest()


class OpenCVCamera(CameraInterface):
    """OpenCV-based USB camera implementation."""
    
    def __init__(self, config: CameraConfig) -> None:
        """Initialize OpenCV camera."""
        super().__init__(config)
        self.cap: Optional[cv2.VideoCapture] = None
        
    def _validate_device_id(self) -> int:
        """Validate and convert device ID to integer."""
        try:
            device_id = int(self.config.device_id)
            if device_id < 0:
                raise ValueError(f"Device ID must be non-negative, got {device_id}")
            return device_id
        except (ValueError, TypeError) as e:
            raise CameraError(f"Invalid device ID: {self.config.device_id}") from e
        
    def _capture_loop(self) -> None:
        """Capture frames from OpenCV VideoCapture."""
        device_id = self._validate_device_id()
        
        try:
            self.cap = cv2.VideoCapture(device_id)
            
            if not self.cap.isOpened():
                raise CameraError(f"Failed to open camera {device_id}")
            
            # Set camera properties
            self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, self.config.width)
            self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, self.config.height)
            self.cap.set(cv2.CAP_PROP_FPS, self.config.fps)
            
            # Reduce buffer size to minimize latency
            self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
            
            consecutive_failures = 0
            max_consecutive_failures = 10
            
            while self.is_running:
                ret, frame = self.cap.read()
                if ret and frame is not None:
                    timestamp = time.time()
                    self.frame_buffer.put(frame, timestamp)
                    consecutive_failures = 0
                else:
                    consecutive_failures += 1
                    logger.warning(
                        f"Failed to read frame from {self.config.name} "
                        f"(failure {consecutive_failures}/{max_consecutive_failures})"
                    )
                    
                    if consecutive_failures >= max_consecutive_failures:
                        logger.error(f"Too many consecutive failures for {self.config.name}")
                        break
                        
                    time.sleep(0.001)
                    
        except Exception as e:
            logger.error(f"Camera {self.config.name} error: {e}")
            raise
        finally:
            if self.cap is not None:
                self.cap.release()
                self.cap = None


class RealSenseCamera(CameraInterface):
    """Intel RealSense depth camera implementation."""
    
    def __init__(self, config: CameraConfig) -> None:
        """Initialize RealSense camera."""
        super().__init__(config)
        self.pipeline = None
        self.align = None
        
    def _capture_loop(self) -> None:
        """Capture frames from RealSense camera."""
        try:
            import pyrealsense2 as rs
        except ImportError as e:
            logger.error("pyrealsense2 not installed. Install with: pip install pyrealsense2")
            raise CameraError("RealSense library not available") from e
        
        try:
            self.pipeline = rs.pipeline()
            config = rs.config()
            
            # Configure streams
            config.enable_stream(
                rs.stream.color, 
                self.config.width, 
                self.config.height, 
                rs.format.bgr8, 
                self.config.fps
            )
            config.enable_stream(
                rs.stream.depth, 
                self.config.width, 
                self.config.height,
                rs.format.z16, 
                self.config.fps
            )
            
            # Start pipeline
            self.pipeline.start(config)
            
            # Align depth to color
            align_to = rs.stream.color
            self.align = rs.align(align_to)
            
            while self.is_running:
                frames = self.pipeline.wait_for_frames(timeout_ms=1000)
                if not frames:
                    continue
                    
                aligned_frames = self.align.process(frames)
                
                color_frame = aligned_frames.get_color_frame()
                depth_frame = aligned_frames.get_depth_frame()
                
                if color_frame and depth_frame:
                    timestamp = time.time()
                    
                    # Convert to numpy arrays
                    color_image = np.asanyarray(color_frame.get_data())
                    depth_image = np.asanyarray(depth_frame.get_data())
                    
                    # Stack RGB and depth (4 channels total)
                    depth_normalized = depth_image.astype(np.uint8)
                    rgbd_image = np.dstack((color_image, depth_normalized[:,:,np.newaxis]))
                    
                    self.frame_buffer.put(rgbd_image, timestamp)
                    
        except Exception as e:
            logger.error(f"RealSense camera {self.config.name} error: {e}")
            raise
        finally:
            if self.pipeline:
                self.pipeline.stop()
                self.pipeline = None


class IPCamera(CameraInterface):
    """IP/Network camera implementation."""
    
    def __init__(self, config: CameraConfig) -> None:
        """Initialize IP camera."""
        super().__init__(config)
        if not isinstance(config.device_id, str):
            raise ValueError(f"IP camera requires string URL, got {type(config.device_id)}")
        self.stream_url = str(config.device_id)
        self.cap: Optional[cv2.VideoCapture] = None
        
    def _validate_url(self, url: str) -> None:
        """Validate stream URL."""
        if not url:
            raise ValueError("Stream URL cannot be empty")
        if not (url.startswith('rtsp://') or url.startswith('http://') or url.startswith('https://')):
            raise ValueError(f"Invalid stream URL protocol: {url}")
        
    def _capture_loop(self) -> None:
        """Capture frames from IP camera stream."""
        self._validate_url(self.stream_url)
        
        reconnect_attempts = 0
        
        while self.is_running and reconnect_attempts < self.config.max_reconnect_attempts:
            try:
                self.cap = cv2.VideoCapture(self.stream_url)
                
                if not self.cap.isOpened():
                    raise CameraError(f"Failed to connect to {self.stream_url}")
                
                # Set buffer size to reduce latency
                self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)
                
                consecutive_failures = 0
                max_consecutive_failures = 10
                
                while self.is_running:
                    ret, frame = self.cap.read()
                    if ret and frame is not None:
                        timestamp = time.time()
                        self.frame_buffer.put(frame, timestamp)
                        consecutive_failures = 0
                        reconnect_attempts = 0  # Reset on success
                    else:
                        consecutive_failures += 1
                        logger.warning(
                            f"Failed to read frame from IP camera {self.config.name} "
                            f"(failure {consecutive_failures}/{max_consecutive_failures})"
                        )
                        
                        if consecutive_failures >= max_consecutive_failures:
                            raise CameraError("Too many consecutive read failures")
                            
            except Exception as e:
                logger.error(f"IP camera {self.config.name} error: {e}")
                reconnect_attempts += 1
                
                if self.cap:
                    self.cap.release()
                    self.cap = None
                    
                if reconnect_attempts < self.config.max_reconnect_attempts:
                    logger.info(
                        f"Attempting reconnection {reconnect_attempts}/{self.config.max_reconnect_attempts}"
                    )
                    time.sleep(self.config.reconnect_delay)
                else:
                    logger.error(f"Max reconnection attempts reached for {self.config.name}")
                    break
                    
        if self.cap:
            self.cap.release()
            self.cap = None


class CameraManager:
    """Manages multiple cameras for robot arm system."""
    
    def __init__(self, config_file: Optional[str] = None) -> None:
        """
        Initialize camera manager.
        
        Args:
            config_file: Optional path to YAML configuration file
        """
        self.cameras: Dict[str, CameraInterface] = {}
        self.recording_enabled = False
        self.recording_path: Optional[Path] = None
        self.video_writers: Dict[str, cv2.VideoWriter] = {}
        self._lock = threading.RLock()
        self._recording_lock = threading.Lock()
        
        if config_file:
            self.load_config(config_file)
    
    def _validate_path(self, path: Union[str, Path]) -> Path:
        """
        Validate and sanitize file path.
        
        Args:
            path: Path to validate
            
        Returns:
            Validated Path object
            
        Raises:
            ValueError: If path is invalid or contains traversal attempts
        """
        path_obj = Path(path).resolve()
        
        # Check for path traversal attempts
        try:
            path_obj.relative_to(Path.cwd())
        except ValueError:
            # Path is outside current directory, check if it's absolute and safe
            if not path_obj.is_absolute():
                raise ValueError(f"Invalid path: {path}")
                
        return path_obj
    
    def load_config(self, config_file: str) -> None:
        """
        Load camera configuration from YAML file.
        
        Args:
            config_file: Path to YAML configuration file
        """
        config_path = self._validate_path(config_file)
        
        if not config_path.exists():
            raise FileNotFoundError(f"Configuration file not found: {config_path}")
            
        try:
            with open(config_path, 'r') as f:
                config = yaml.safe_load(f)
        except yaml.YAMLError as e:
            raise ValueError(f"Invalid YAML configuration: {e}") from e
        
        if not isinstance(config, dict):
            raise ValueError("Configuration must be a dictionary")
            
        for cam_name, cam_config in config.get('cameras', {}).items():
            if not isinstance(cam_config, dict):
                logger.warning(f"Skipping invalid camera config: {cam_name}")
                continue
                
            try:
                self.add_camera(
                    name=cam_name,
                    camera_type=cam_config.get('type', 'opencv'),
                    device_id=cam_config.get('index_or_path', 0),
                    width=cam_config.get('width', 1280),
                    height=cam_config.get('height', 720),
                    fps=cam_config.get('fps', 30),
                    mount_position=cam_config.get('mount', 'wrist')
                )
            except Exception as e:
                logger.error(f"Failed to add camera {cam_name}: {e}")
    
    def add_camera(
        self, 
        name: str, 
        camera_type: str, 
        device_id: Union[int, str],
        width: int = 1280, 
        height: int = 720, 
        fps: int = 30,
        mount_position: str = "wrist"
    ) -> None:
        """
        Add a camera to the manager.
        
        Args:
            name: Unique camera name
            camera_type: Type of camera (opencv, realsense, ip)
            device_id: Device identifier (index for USB, URL for IP)
            width: Frame width in pixels
            height: Frame height in pixels
            fps: Frames per second
            mount_position: Camera mounting position
        """
        if not name:
            raise ValueError("Camera name cannot be empty")
        if name in self.cameras:
            raise ValueError(f"Camera {name} already exists")
            
        # Validate parameters
        if width <= 0 or height <= 0:
            raise ValueError(f"Invalid resolution: {width}x{height}")
        if fps <= 0:
            raise ValueError(f"Invalid FPS: {fps}")
        
        # Create camera configuration
        try:
            camera_type_enum = CameraType(camera_type.lower())
        except ValueError:
            raise ValueError(f"Unknown camera type: {camera_type}")
            
        config = CameraConfig(
            name=name,
            type=camera_type_enum,
            device_id=device_id,
            width=width,
            height=height,
            fps=fps,
            mount_position=mount_position
        )
        
        # Create appropriate camera instance
        if config.type in (CameraType.OPENCV, CameraType.USB):
            camera = OpenCVCamera(config)
        elif config.type == CameraType.REALSENSE:
            camera = RealSenseCamera(config)
        elif config.type == CameraType.IP:
            camera = IPCamera(config)
        else:
            raise ValueError(f"Unsupported camera type: {config.type}")
        
        with self._lock:
            self.cameras[name] = camera
            
        logger.info(f"Added camera {name} ({camera_type}) at {mount_position}")
    
    def start_all(self) -> None:
        """Start all cameras."""
        with self._lock:
            for camera in self.cameras.values():
                try:
                    camera.start()
                except Exception as e:
                    logger.error(f"Failed to start camera {camera.config.name}: {e}")
    
    def stop_all(self) -> None:
        """Stop all cameras."""
        # Stop recording first if active
        if self.recording_enabled:
            self.stop_recording()
            
        with self._lock:
            for camera in self.cameras.values():
                try:
                    camera.stop()
                except Exception as e:
                    logger.error(f"Error stopping camera {camera.config.name}: {e}")
    
    def get_frames(self) -> Dict[str, npt.NDArray[np.uint8]]:
        """
        Get latest frames from all cameras.
        
        Returns:
            Dictionary mapping camera names to frames
        """
        frames: Dict[str, npt.NDArray[np.uint8]] = {}
        
        with self._lock:
            for name, camera in self.cameras.items():
                frame = camera.get_frame()
                if frame is not None:
                    frames[name] = frame
                    
        return frames
    
    def get_synchronized_frames(
        self, 
        timestamp: float, 
        tolerance: float = 0.05
    ) -> Dict[str, npt.NDArray[np.uint8]]:
        """
        Get frames from all cameras synchronized to a specific timestamp.
        
        Args:
            timestamp: Target timestamp for synchronization
            tolerance: Maximum time difference allowed
            
        Returns:
            Dictionary mapping camera names to synchronized frames
        """
        if tolerance <= 0:
            raise ValueError(f"Tolerance must be positive, got {tolerance}")
            
        frames: Dict[str, npt.NDArray[np.uint8]] = {}
        
        with self._lock:
            for name, camera in self.cameras.items():
                frame = camera.frame_buffer.get_synchronized(timestamp, tolerance)
                if frame is not None:
                    frames[name] = frame
                    
        return frames
    
    def start_recording(self, output_dir: str, episode_id: str) -> None:
        """
        Start recording video from all cameras.
        
        Args:
            output_dir: Directory to save recordings
            episode_id: Unique episode identifier
        """
        with self._recording_lock:
            if self.recording_enabled:
                raise ValueError("Recording already in progress")
                
            # Validate and create output directory
            output_path = self._validate_path(output_dir)
            self.recording_path = output_path / episode_id
            self.recording_path.mkdir(parents=True, exist_ok=True)
            
            with self._lock:
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
                    
                    if not writer.isOpened():
                        # Clean up any writers that were created
                        for w in self.video_writers.values():
                            w.release()
                        self.video_writers.clear()
                        raise CameraError(f"Failed to create video writer for {name}")
                        
                    self.video_writers[name] = writer
            
            self.recording_enabled = True
            logger.info(f"Started recording to {self.recording_path}")
    
    def stop_recording(self) -> None:
        """Stop recording video."""
        with self._recording_lock:
            if not self.recording_enabled:
                return
                
            self.recording_enabled = False
            
            with self._lock:
                for writer in self.video_writers.values():
                    try:
                        writer.release()
                    except Exception as e:
                        logger.error(f"Error releasing video writer: {e}")
                        
                self.video_writers.clear()
                
            logger.info("Stopped recording")
    
    def save_frame_batch(
        self, 
        frames: Dict[str, npt.NDArray[np.uint8]], 
        timestamp: float
    ) -> None:
        """
        Save a batch of frames with timestamp (for dataset creation).
        
        Args:
            frames: Dictionary mapping camera names to frames
            timestamp: Timestamp for the frame batch
        """
        if not self.recording_enabled:
            return
            
        with self._lock:
            for name, frame in frames.items():
                if name in self.video_writers:
                    try:
                        self.video_writers[name].write(frame)
                    except Exception as e:
                        logger.error(f"Error writing frame for {name}: {e}")
    
    def __enter__(self) -> 'CameraManager':
        """Context manager entry."""
        return self
        
    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit - ensure cleanup."""
        self.stop_all()


class CameraCalibration:
    """Camera calibration utilities for SO-101 arms."""
    
    @staticmethod
    def calibrate_intrinsics(
        camera: CameraInterface, 
        checkerboard_size: Tuple[int, int] = (9, 6),
        num_frames: int = 30
    ) -> Dict[str, Any]:
        """
        Calibrate camera intrinsics using checkerboard pattern.
        
        Args:
            camera: Camera to calibrate
            checkerboard_size: Number of inner corners (width, height)
            num_frames: Number of calibration frames to capture
            
        Returns:
            Dictionary containing calibration parameters
        """
        if num_frames <= 0:
            raise ValueError(f"num_frames must be positive, got {num_frames}")
            
        criteria = (cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 30, 0.001)
        
        # Prepare object points
        objp = np.zeros((checkerboard_size[0] * checkerboard_size[1], 3), np.float32)
        objp[:, :2] = np.mgrid[0:checkerboard_size[0], 0:checkerboard_size[1]].T.reshape(-1, 2)
        
        objpoints: List[npt.NDArray[np.float32]] = []
        imgpoints: List[npt.NDArray[np.float32]] = []
        
        logger.info(f"Calibrating {camera.config.name}. Show checkerboard pattern...")
        
        frames_captured = 0
        gray_shape = None
        
        while frames_captured < num_frames:
            frame = camera.get_frame()
            if frame is None:
                time.sleep(0.1)
                continue
            
            gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
            gray_shape = gray.shape
            ret, corners = cv2.findChessboardCorners(gray, checkerboard_size, None)
            
            if ret:
                objpoints.append(objp)
                corners2 = cv2.cornerSubPix(gray, corners, (11, 11), (-1, -1), criteria)
                imgpoints.append(corners2)
                frames_captured += 1
                
                # Draw and display
                cv2.drawChessboardCorners(frame, checkerboard_size, corners2, ret)
                cv2.putText(
                    frame, 
                    f"Captured: {frames_captured}/{num_frames}", 
                    (10, 30), 
                    cv2.FONT_HERSHEY_SIMPLEX, 
                    1, 
                    (0, 255, 0), 
                    2
                )
                logger.info(f"Captured calibration frame {frames_captured}/{num_frames}")
            
            cv2.imshow(f"Calibration - {camera.config.name}", frame)
            if cv2.waitKey(100) & 0xFF == ord('q'):
                logger.info("Calibration cancelled by user")
                break
        
        cv2.destroyAllWindows()
        
        if len(objpoints) < 10:
            raise CameraError(f"Insufficient calibration frames: {len(objpoints)}")
        
        # Calibrate
        if gray_shape is None:
            raise CameraError("No frames captured for calibration")
            
        ret, mtx, dist, rvecs, tvecs = cv2.calibrateCamera(
            objpoints, imgpoints, gray_shape[::-1], None, None
        )
        
        return {
            'camera_matrix': mtx.tolist(),
            'distortion_coeffs': dist.tolist(),
            'calibration_error': float(ret),
            'num_frames_used': len(objpoints)
        }
    
    @staticmethod
    def save_calibration(calibration_data: Dict[str, Any], output_file: str) -> None:
        """
        Save calibration data to YAML file.
        
        Args:
            calibration_data: Calibration parameters to save
            output_file: Path to output YAML file
        """
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, 'w') as f:
            yaml.dump(calibration_data, f, default_flow_style=False)
            
        logger.info(f"Saved calibration to {output_path}")
    
    @staticmethod
    def load_calibration(calibration_file: str) -> Dict[str, Any]:
        """
        Load calibration data from YAML file.
        
        Args:
            calibration_file: Path to calibration YAML file
            
        Returns:
            Dictionary containing calibration parameters
        """
        calib_path = Path(calibration_file)
        
        if not calib_path.exists():
            raise FileNotFoundError(f"Calibration file not found: {calib_path}")
            
        with open(calib_path, 'r') as f:
            return yaml.safe_load(f)


# Example usage for async integration with robot control
async def camera_robot_sync_example() -> None:
    """Example of synchronizing camera capture with robot control loop."""
    
    # Initialize camera manager
    cam_manager = CameraManager()
    
    try:
        cam_manager.add_camera("wrist", "opencv", 0)
        cam_manager.add_camera("table", "opencv", 1)
        cam_manager.start_all()
        
        # Simulated robot control loop at 200Hz
        control_rate = 200  # Hz
        dt = 1.0 / control_rate
        
        for _ in range(1000):  # Run for ~5 seconds
            loop_start = time.time()
            
            # Get current timestamp for synchronization
            current_time = time.time()
            
            # Get synchronized frames (will use closest frames within tolerance)
            frames = cam_manager.get_synchronized_frames(current_time, tolerance=0.02)
            
            # Process frames for VLA model (async)
            if frames:
                # Prepare observation for policy
                # observation = {
                #     'images': frames,
                #     'timestamp': current_time
                # }
                # Here you would send to policy server asynchronously
                # action = await policy_client.get_action(observation)
                pass
            
            # Sleep to maintain control rate
            elapsed = time.time() - loop_start
            if elapsed < dt:
                await asyncio.sleep(dt - elapsed)
                
    except Exception as e:
        logger.error(f"Error in camera-robot sync: {e}")
    finally:
        cam_manager.stop_all()


def main() -> None:
    """Main function for testing camera manager."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    
    with CameraManager() as manager:
        try:
            manager.add_camera("test_cam", "opencv", 0)
            manager.start_all()
            
            logger.info("Camera manager started. Press 'q' to quit.")
            
            for i in range(300):  # Run for ~10 seconds at 30 FPS
                frames = manager.get_frames()
                if "test_cam" in frames:
                    cv2.imshow("Test Camera", frames["test_cam"])
                    if cv2.waitKey(33) & 0xFF == ord('q'):
                        break
                time.sleep(0.033)  # ~30 FPS
                
        except Exception as e:
            logger.error(f"Error in main: {e}")
        finally:
            cv2.destroyAllWindows()


if __name__ == "__main__":
    main()