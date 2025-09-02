#!/usr/bin/env python3
"""
VLA Model Client for SO-101 robotic arms.
Connects camera feeds to remote VLA models (π0-FAST/OpenVLA).
"""

import asyncio
import numpy as np
import aiohttp
import cv2
import json
import time
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
import logging

logger = logging.getLogger(__name__)

@dataclass
class VLAConfig:
    """Configuration for VLA model inference."""
    model_type: str = "pi0_fast"  # or "openvla"
    server_url: str = "http://localhost:8000"
    action_chunk_size: int = 64
    observation_history: int = 3
    image_size: Tuple[int, int] = (224, 224)
    timeout: float = 0.1  # 100ms max latency

class VLAClient:
    """Client for remote VLA model inference."""
    
    def __init__(self, config: VLAConfig):
        self.config = config
        self.session = None
        self.observation_buffer = []
        self.action_buffer = []
        self.last_inference_time = 0
        
    async def initialize(self):
        """Initialize connection to model server."""
        self.session = aiohttp.ClientSession()
        
        # Test connection
        try:
            async with self.session.get(f"{self.config.server_url}/health") as resp:
                if resp.status == 200:
                    logger.info(f"Connected to VLA server at {self.config.server_url}")
                else:
                    logger.error(f"VLA server unhealthy: {resp.status}")
        except Exception as e:
            logger.error(f"Failed to connect to VLA server: {e}")
            
    async def preprocess_observation(self, frames: Dict[str, np.ndarray], 
                                    robot_state: Dict) -> Dict:
        """Preprocess camera frames and robot state for VLA input."""
        processed = {
            'images': {},
            'proprioception': [],
            'timestamp': time.time()
        }
        
        # Process camera frames
        for cam_name, frame in frames.items():
            if frame is not None:
                # Resize to model input size
                resized = cv2.resize(frame, self.config.image_size)
                # Normalize to [0, 1]
                normalized = resized.astype(np.float32) / 255.0
                processed['images'][cam_name] = normalized.tolist()
                
        # Add robot proprioception
        if robot_state:
            processed['proprioception'] = [
                robot_state.get('joint_positions', []),
                robot_state.get('joint_velocities', []),
                [robot_state.get('gripper_position', 0.0)]
            ]
            
        return processed
        
    async def infer_action(self, observation: Dict) -> Optional[np.ndarray]:
        """Send observation to VLA server and get action."""
        if not self.session:
            return None
            
        # Add to observation buffer
        self.observation_buffer.append(observation)
        if len(self.observation_buffer) > self.config.observation_history:
            self.observation_buffer.pop(0)
            
        # Prepare request
        request_data = {
            'observations': self.observation_buffer,
            'model': self.config.model_type,
            'action_chunk_size': self.config.action_chunk_size
        }
        
        try:
            async with self.session.post(
                f"{self.config.server_url}/infer",
                json=request_data,
                timeout=aiohttp.ClientTimeout(total=self.config.timeout)
            ) as resp:
                if resp.status == 200:
                    result = await resp.json()
                    actions = np.array(result['actions'])
                    
                    # Store action chunk
                    self.action_buffer = actions
                    self.last_inference_time = time.time()
                    
                    return actions
                else:
                    logger.error(f"Inference failed: {resp.status}")
                    return None
                    
        except asyncio.TimeoutError:
            logger.warning("VLA inference timeout")
            return None
        except Exception as e:
            logger.error(f"Inference error: {e}")
            return None
            
    def get_next_action(self) -> Optional[np.ndarray]:
        """Get next action from buffer (for 200Hz control)."""
        if len(self.action_buffer) > 0:
            # Pop first action from chunk
            action = self.action_buffer[0]
            self.action_buffer = self.action_buffer[1:]
            return action
        return None
        
    async def cleanup(self):
        """Clean up resources."""
        if self.session:
            await self.session.close()
