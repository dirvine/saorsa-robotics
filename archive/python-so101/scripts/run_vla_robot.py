#!/usr/bin/env python3
"""
Main script to run SO-101 robot with VLA control.
"""

import asyncio
import yaml
import os
import sys
from pathlib import Path

sys.path.append(str(Path(__file__).parent.parent))

from robot.camera_manager import CameraManager, CameraConfig, CameraType
from robot.vla_client import VLAClient, VLAConfig

async def main():
    # Load configurations
    with open('robot/configs/camera_config.yaml', 'r') as f:
        camera_config = yaml.safe_load(f)
    
    with open('robot/configs/vla_config.yaml', 'r') as f:
        vla_config = yaml.safe_load(f)
    
    # Initialize camera manager
    camera_configs = []
    for cam_name, cam_cfg in camera_config['cameras'].items():
        camera_configs.append(CameraConfig(
            name=cam_name,
            type=CameraType(cam_cfg['type']),
            device_id=cam_cfg.get('device_id', 0),
            width=cam_cfg['resolution'][0],
            height=cam_cfg['resolution'][1],
            fps=cam_cfg['fps']
        ))
    
    camera_manager = CameraManager(camera_configs)
    await camera_manager.initialize()
    print(f"Initialized {len(camera_configs)} cameras")
    
    # Initialize VLA client
    vla_client_config = VLAConfig(
        model_type=vla_config['vla']['model_type'],
        server_url=os.getenv('VLA_SERVER_URL', 'http://localhost:8000'),
        action_chunk_size=vla_config['vla']['action_chunk_size'],
        observation_history=vla_config['vla']['observation_history'],
        image_size=tuple(vla_config['vla']['image_size'])
    )
    
    vla_client = VLAClient(vla_client_config)
    await vla_client.initialize()
    print("Connected to VLA server")
    
    print("\nVLA Robot Control Active")
    print("Press Ctrl+C to stop")
    
    try:
        while True:
            # Get frames
            frames = await camera_manager.get_frames()
            
            # Process and infer
            obs = await vla_client.preprocess_observation(
                {k: v['color'] for k, v in frames.items() if v},
                {'joint_positions': [0]*7}
            )
            
            action = await vla_client.infer_action(obs)
cat >> Makefile << 'EOF'

# VLA commands
run-vla-server:
	@echo "Starting VLA model server..."
	cd policy_server && python vla_server.py

run-vla-robot:
	@echo "Starting VLA-controlled robot..."
	python scripts/run_vla_robot.py

install-vla-deps:
	@echo "Installing VLA dependencies..."
	pip install fastapi uvicorn aiohttp pydantic

.PHONY: run-vla-server run-vla-robot install-vla-deps
