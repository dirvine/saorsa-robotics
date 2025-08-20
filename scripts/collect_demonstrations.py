#!/usr/bin/env python3
"""
Collect demonstration data for VLA model training.

Synchronized camera and robot state recording for SO-101 arms.
"""

import asyncio
import aiofiles
import cv2
import numpy as np
import numpy.typing as npt
import json
import time
import pickle
import logging
from pathlib import Path
from datetime import datetime
import yaml
from typing import Dict, List, Optional, Any, Tuple
import sys
import threading
from concurrent.futures import ThreadPoolExecutor

# Add parent directory to path for imports
sys.path.append(str(Path(__file__).parent.parent))

from robot.camera_manager import CameraManager

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class DemonstrationError(Exception):
    """Custom exception for demonstration collection errors."""
    pass


class DemonstrationCollector:
    """Collects synchronized demonstration data from cameras and robot."""
    
    def __init__(self, config_path: str = "robot/configs/camera_config.yaml") -> None:
        """
        Initialize demonstration collector.
        
        Args:
            config_path: Path to camera configuration file
        """
        self.config_path = Path(config_path)
        if not self.config_path.exists():
            raise FileNotFoundError(f"Configuration file not found: {config_path}")
            
        try:
            with open(self.config_path, 'r') as f:
                self.config = yaml.safe_load(f)
        except yaml.YAMLError as e:
            raise DemonstrationError(f"Failed to load config: {e}") from e
            
        # Validate and create output directory
        output_dir_str = self.config.get('recording', {}).get('output_dir', 'data/demonstrations')
        self.output_dir = Path(output_dir_str)
        
        # Security check for path traversal
        try:
            self.output_dir = self.output_dir.resolve()
            self.output_dir.relative_to(Path.cwd())
        except ValueError:
            if not self.output_dir.is_absolute():
                raise ValueError(f"Invalid output directory: {output_dir_str}")
                
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        self.camera_manager: Optional[CameraManager] = None
        self.recording = False
        self.session_id = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.current_episode: Dict[str, Any] = {}
        self.metadata: Dict[str, Any] = {
            'session_id': self.session_id,
            'config': self.config,
            'episodes': []
        }
        self._lock = threading.RLock()
        self._executor = ThreadPoolExecutor(max_workers=4)
        
    async def initialize(self) -> None:
        """Initialize camera manager with configuration."""
        try:
            self.camera_manager = CameraManager()
            
            # Add cameras from config
            for cam_name, cam_cfg in self.config.get('cameras', {}).items():
                if not isinstance(cam_cfg, dict):
                    logger.warning(f"Invalid camera config for {cam_name}")
                    continue
                    
                try:
                    self.camera_manager.add_camera(
                        name=cam_name,
                        camera_type=cam_cfg.get('type', 'opencv'),
                        device_id=cam_cfg.get('index_or_path', 0),
                        width=cam_cfg.get('width', 1280),
                        height=cam_cfg.get('height', 720),
                        fps=cam_cfg.get('fps', 30),
                        mount_position=cam_cfg.get('mount', 'wrist')
                    )
                except Exception as e:
                    logger.error(f"Failed to add camera {cam_name}: {e}")
                    
            # Start all cameras
            self.camera_manager.start_all()
            
            logger.info(f"Initialized {len(self.camera_manager.cameras)} cameras")
            
        except Exception as e:
            raise DemonstrationError(f"Failed to initialize cameras: {e}") from e
        
    async def start_episode(self, task_name: str = "task") -> int:
        """
        Start recording a new episode.
        
        Args:
            task_name: Name/description of the task being demonstrated
            
        Returns:
            Episode ID
        """
        with self._lock:
            if self.recording:
                raise ValueError("Episode already in progress")
                
            self.recording = True
            episode_id = len(self.metadata['episodes'])
            timestamp = datetime.now().isoformat()
            
            self.current_episode = {
                'episode_id': episode_id,
                'task': task_name,
                'start_time': timestamp,
                'frames': [],
                'actions': [],
                'states': []
            }
            
            # Start video recording if configured
            if self.config.get('save_videos', False):
                episode_dir = f"{self.session_id}/episode_{episode_id:04d}"
                self.camera_manager.start_recording(str(self.output_dir), episode_dir)
            
            logger.info(f"Started episode {episode_id}: {task_name}")
            return episode_id
        
    async def record_step(
        self, 
        robot_state: Dict[str, Any], 
        action: Optional[List[float]] = None
    ) -> None:
        """
        Record a single step with synchronized data.
        
        Args:
            robot_state: Current robot state dictionary
            action: Optional action taken at this step
        """
        if not self.recording:
            return
            
        if not isinstance(robot_state, dict):
            raise TypeError("robot_state must be a dictionary")
            
        # Get synchronized frames from all cameras
        timestamp = time.time()
        frames = self.camera_manager.get_synchronized_frames(timestamp, tolerance=0.02)
        
        if not frames:
            logger.warning("No frames available for recording")
            return
            
        step_data = {
            'timestamp': timestamp,
            'frames': {},
            'robot_state': robot_state,
            'action': action
        }
        
        # Process frames asynchronously
        loop = asyncio.get_event_loop()
        
        for cam_name, frame in frames.items():
            if frame is not None:
                # Optionally resize for VLA
                resize_config = self.config.get('vla_integration', {}).get('resize_to')
                if resize_config and len(resize_config) == 2:
                    target_size = tuple(resize_config)
                    frame_resized = await loop.run_in_executor(
                        self._executor,
                        cv2.resize,
                        frame,
                        target_size
                    )
                else:
                    frame_resized = frame
                    
                step_data['frames'][cam_name] = {
                    'data': frame_resized,
                    'shape': frame_resized.shape,
                    'dtype': str(frame_resized.dtype),
                    'timestamp': timestamp
                }
                
        with self._lock:
            if self.recording and self.current_episode:
                self.current_episode['frames'].append(step_data['frames'])
                self.current_episode['states'].append(robot_state)
                if action is not None:
                    self.current_episode['actions'].append(action)
            
    async def end_episode(self) -> None:
        """End current episode and save data."""
        with self._lock:
            if not self.recording:
                logger.warning("No episode in progress")
                return
                
            self.recording = False
            
            if not self.current_episode:
                logger.warning("No episode data to save")
                return
                
            self.current_episode['end_time'] = datetime.now().isoformat()
            
            # Stop video recording
            if self.camera_manager and self.config.get('save_videos', False):
                self.camera_manager.stop_recording()
        
        # Save episode data (do this outside the lock to avoid blocking)
        await self._save_episode_data()
            
    async def _save_episode_data(self) -> None:
        """Save episode data to disk."""
        if not self.current_episode:
            return
            
        episode_id = self.current_episode['episode_id']
        episode_dir = self.output_dir / self.session_id / f"episode_{episode_id:04d}"
        episode_dir.mkdir(parents=True, exist_ok=True)
        
        try:
            # Save as pickle for efficient loading
            episode_path = episode_dir / "episode_data.pkl"
            
            # Use async file I/O
            loop = asyncio.get_event_loop()
            await loop.run_in_executor(
                self._executor,
                self._save_pickle,
                episode_path,
                self.current_episode
            )
            
            # Update and save metadata
            with self._lock:
                self.metadata['episodes'].append({
                    'episode_id': episode_id,
                    'task': self.current_episode['task'],
                    'start_time': self.current_episode['start_time'],
                    'end_time': self.current_episode['end_time'],
                    'num_steps': len(self.current_episode.get('frames', [])),
                    'path': str(episode_path.relative_to(self.output_dir))
                })
            
            metadata_path = self.output_dir / self.session_id / "metadata.json"
            async with aiofiles.open(metadata_path, 'w') as f:
                await f.write(json.dumps(self.metadata, indent=2))
                
            logger.info(
                f"Saved episode {episode_id} with "
                f"{len(self.current_episode.get('frames', []))} steps"
            )
            
        except Exception as e:
            logger.error(f"Failed to save episode data: {e}")
            raise DemonstrationError(f"Failed to save episode: {e}") from e
        finally:
            self.current_episode = {}
            
    def _save_pickle(self, path: Path, data: Any) -> None:
        """Helper to save pickle file (for executor)."""
        with open(path, 'wb') as f:
            pickle.dump(data, f, protocol=pickle.HIGHEST_PROTOCOL)
        
    async def cleanup(self) -> None:
        """Clean up resources."""
        try:
            # End any ongoing episode
            if self.recording:
                await self.end_episode()
                
            # Stop cameras
            if self.camera_manager:
                self.camera_manager.stop_all()
                
            # Shutdown executor
            self._executor.shutdown(wait=True)
            
            logger.info("Cleanup completed")
            
        except Exception as e:
            logger.error(f"Error during cleanup: {e}")


async def interactive_collection_loop(collector: DemonstrationCollector) -> None:
    """
    Interactive demonstration collection with keyboard control.
    
    Args:
        collector: DemonstrationCollector instance
    """
    import aioconsole
    
    logger.info("\nDemonstration Collection Controls:")
    logger.info("  s - Start new episode")
    logger.info("  e - End current episode")
    logger.info("  q - Quit")
    logger.info("\nPress 's' to start recording...")
    
    recording = False
    control_task = None
    
    async def control_loop():
        """Background control loop for recording."""
        nonlocal recording
        
        while True:
            if recording and collector.recording:
                # Simulate robot state (replace with actual robot state)
                robot_state = {
                    'joint_positions': np.random.randn(7).tolist(),
                    'joint_velocities': np.zeros(7).tolist(),
                    'gripper_position': 0.0,
                    'timestamp': time.time()
                }
                
                # Simulate action (replace with actual control)
                action = np.random.randn(7).tolist()
                
                try:
                    await collector.record_step(robot_state, action)
                except Exception as e:
                    logger.error(f"Error recording step: {e}")
                    
            await asyncio.sleep(0.005)  # 200Hz
    
    # Start control loop
    control_task = asyncio.create_task(control_loop())
    
    try:
        while True:
            key = await aioconsole.ainput()
            
            if key.lower() == 's' and not recording:
                task_name = await aioconsole.ainput("Enter task name: ")
                await collector.start_episode(task_name or "demonstration")
                recording = True
                logger.info("Recording started")
                
            elif key.lower() == 'e' and recording:
                await collector.end_episode()
                recording = False
                logger.info("Recording stopped")
                
            elif key.lower() == 'q':
                if recording:
                    await collector.end_episode()
                break
                
    except Exception as e:
        logger.error(f"Error in control loop: {e}")
    finally:
        if control_task:
            control_task.cancel()
            try:
                await control_task
            except asyncio.CancelledError:
                pass


async def main() -> None:
    """Main function for demonstration collection."""
    # Parse command line arguments
    import argparse
    
    parser = argparse.ArgumentParser(
        description='Collect demonstrations for SO-101 robot arms'
    )
    parser.add_argument(
        '--config',
        type=str,
        default='robot/configs/camera_config.yaml',
        help='Path to camera configuration file'
    )
    parser.add_argument(
        '--interactive',
        action='store_true',
        help='Run in interactive mode with keyboard control'
    )
    
    args = parser.parse_args()
    
    try:
        collector = DemonstrationCollector(config_path=args.config)
        await collector.initialize()
        
        if args.interactive:
            await interactive_collection_loop(collector)
        else:
            # Non-interactive mode - record for fixed duration
            logger.info("Starting automatic collection for 30 seconds...")
            
            await collector.start_episode("automatic_demonstration")
            
            for _ in range(6000):  # 30 seconds at 200Hz
                robot_state = {
                    'joint_positions': np.random.randn(7).tolist(),
                    'joint_velocities': np.zeros(7).tolist(),
                    'gripper_position': 0.0,
                    'timestamp': time.time()
                }
                action = np.random.randn(7).tolist()
                
                await collector.record_step(robot_state, action)
                await asyncio.sleep(0.005)
                
            await collector.end_episode()
            logger.info("Automatic collection completed")
            
    except KeyboardInterrupt:
        logger.info("Collection interrupted by user")
    except Exception as e:
        logger.error(f"Error during collection: {e}")
        raise
    finally:
        if 'collector' in locals():
            await collector.cleanup()
        logger.info("Collection stopped")


if __name__ == "__main__":
    asyncio.run(main())