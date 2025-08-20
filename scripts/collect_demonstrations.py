#!/usr/bin/env python3
"""
Collect demonstration data for VLA model training.
Synchronized camera and robot state recording for SO-101 arms.
"""

import asyncio
import cv2
import numpy as np
import json
import time
import pickle
from pathlib import Path
from datetime import datetime
import yaml
import sys
sys.path.append(str(Path(__file__).parent.parent))

from robot.camera_manager import CameraManager, CameraConfig, CameraType

class DemonstrationCollector:
    def __init__(self, config_path="robot/configs/camera_config.yaml"):
        with open(config_path, 'r') as f:
            self.config = yaml.safe_load(f)
            
        self.output_dir = Path(self.config['recording']['output_dir'])
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        self.camera_manager = None
        self.recording = False
        self.session_id = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.current_episode = []
        self.metadata = {
            'session_id': self.session_id,
            'config': self.config,
            'episodes': []
        }
        
    async def initialize(self):
        """Initialize camera manager with config."""
        camera_configs = []
        for cam_name, cam_cfg in self.config['cameras'].items():
            camera_configs.append(CameraConfig(
                name=cam_name,
                type=CameraType(cam_cfg['type']),
                device_id=cam_cfg.get('device_id', cam_cfg.get('serial', '')),
                width=cam_cfg['resolution'][0],
                height=cam_cfg['resolution'][1],
                fps=cam_cfg['fps'],
                mount_position=cam_cfg['position'],
                calibration_file=cam_cfg.get('calibration_file')
            ))
            
        self.camera_manager = CameraManager(camera_configs)
        await self.camera_manager.initialize()
        print(f"Initialized {len(camera_configs)} cameras")
        
    async def start_episode(self, task_name="task"):
        """Start recording a new episode."""
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
        
        print(f"Started episode {episode_id}: {task_name}")
        return episode_id
        
    async def record_step(self, robot_state, action=None):
        """Record a single step with synchronized data."""
        if not self.recording:
            return
            
        # Get synchronized frames from all cameras
        frames = await self.camera_manager.get_frames()
        timestamp = time.time()
        
        step_data = {
            'timestamp': timestamp,
            'frames': {},
            'robot_state': robot_state,
            'action': action
        }
        
        # Process and store frames
        for cam_name, frame_data in frames.items():
            if frame_data is not None:
                # Resize for VLA if configured
                if 'resize_to' in self.config['vla_integration']:
                    target_size = tuple(self.config['vla_integration']['resize_to'])
                    frame_data['color'] = cv2.resize(frame_data['color'], target_size)
                    
                step_data['frames'][cam_name] = {
                    'color': frame_data['color'],
                    'depth': frame_data.get('depth'),
                    'timestamp': frame_data['timestamp']
                }
                
        self.current_episode['frames'].append(step_data['frames'])
        self.current_episode['states'].append(robot_state)
        if action is not None:
            self.current_episode['actions'].append(action)
            
    async def end_episode(self):
        """End current episode and save data."""
        if not self.recording:
            return
            
        self.recording = False
        self.current_episode['end_time'] = datetime.now().isoformat()
        
        # Save episode data
        episode_id = self.current_episode['episode_id']
        episode_dir = self.output_dir / self.session_id / f"episode_{episode_id:04d}"
        episode_dir.mkdir(parents=True, exist_ok=True)
        
        # Save as pickle for efficient loading
        episode_path = episode_dir / "episode_data.pkl"
        with open(episode_path, 'wb') as f:
            pickle.dump(self.current_episode, f)
            
        # Save metadata
        self.metadata['episodes'].append({
            'episode_id': episode_id,
            'task': self.current_episode['task'],
            'start_time': self.current_episode['start_time'],
            'end_time': self.current_episode['end_time'],
            'num_steps': len(self.current_episode['frames']),
            'path': str(episode_path)
        })
        
        metadata_path = self.output_dir / self.session_id / "metadata.json"
        with open(metadata_path, 'w') as f:
            json.dump(self.metadata, f, indent=2)
            
        print(f"Saved episode {episode_id} with {len(self.current_episode['frames'])} steps")
        self.current_episode = []
        
    async def cleanup(self):
        """Clean up resources."""
        if self.camera_manager:
            await self.camera_manager.cleanup()

async def main():
    """Example usage with keyboard control."""
    collector = DemonstrationCollector()
    await collector.initialize()
    
    print("\nDemonstration Collection Controls:")
    print("  s - Start new episode")
    print("  e - End current episode")
    print("  q - Quit")
    print("\nPress 's' to start recording...")
    
    try:
        recording = False
        while True:
            # Simulate robot state (replace with actual robot state)
            robot_state = {
                'joint_positions': np.random.randn(7).tolist(),
                'joint_velocities': np.zeros(7).tolist(),
                'gripper_position': 0.0
            }
            
            # Simulate action (replace with actual control)
            action = np.random.randn(7).tolist() if recording else None
            
            if recording:
                await collector.record_step(robot_state, action)
                
            await asyncio.sleep(0.005)  # 200Hz
            
    except KeyboardInterrupt:
        if collector.recording:
            await collector.end_episode()
        await collector.cleanup()
        print("\nCollection stopped")

if __name__ == "__main__":
    asyncio.run(main())
