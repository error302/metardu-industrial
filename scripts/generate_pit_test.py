#!/usr/bin/env python3
"""
Generate a synthetic pit excavation scenario for testing the design
surface (DesignSurfaceRef::Dem) code path.

Creates two LAS files:
  1. design_surface.las — a flat plane at z=100m (the "design" pit floor)
  2. excavated_pit.las — the same area with a cone-shaped pit dug into it
     (z = 100 - depth, where depth follows a cone shape)

The analytical excavation volume = π * r² * h / 3 (volume of the cone).
The EOM pipeline should compute this as "cut volume" when comparing
the excavated surface against the design surface.

Usage:
  python3 generate_pit_test.py [output_dir]
"""

import struct
import math
import random
import os
import sys

# ── Pit parameters ──
RADIUS = 25.0      # meters
DEPTH = 10.0       # meters (pit depth at center)
BASE_Z = 100.0     # original ground elevation
CENTER_X = 500.0
CENTER_Y = 5000.0
SPACING = 0.5
NOISE_STD = 0.02
EXPECTED_VOLUME = math.pi * RADIUS**2 * DEPTH / 3  # cone volume

HEADER_SIZE = 227
POINT_DATA_RECORD_LENGTH = 20

def write_las(path, points):
    n = len(points)
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    zs = [p[2] for p in points]
    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)
    min_z, max_z = min(zs), max(zs)
    x_scale = y_scale = z_scale = 0.001
    x_offset, y_offset, z_offset = min_x, min_y, min_z

    header = bytearray(HEADER_SIZE)
    header[0:4] = b'LASF'
    header[26:58] = b'MetaRDU Pit Test'.ljust(32, b'\0')
    header[58:90] = b'generate_pit_test.py'.ljust(32, b'\0')
    struct.pack_into('<H', header, 90, 186)
    struct.pack_into('<H', header, 92, 2026)
    header[24] = 1
    header[25] = 2
    struct.pack_into('<H', header, 94, HEADER_SIZE)
    struct.pack_into('<I', header, 96, HEADER_SIZE)
    struct.pack_into('<I', header, 100, 0)
    header[104] = 0
    struct.pack_into('<H', header, 105, POINT_DATA_RECORD_LENGTH)
    struct.pack_into('<I', header, 107, n)
    struct.pack_into('<I', header, 111, n)
    for i in range(1, 5):
        struct.pack_into('<I', header, 111 + i*4, 0)
    struct.pack_into('<d', header, 131, x_scale)
    struct.pack_into('<d', header, 139, y_scale)
    struct.pack_into('<d', header, 147, z_scale)
    struct.pack_into('<d', header, 155, x_offset)
    struct.pack_into('<d', header, 163, y_offset)
    struct.pack_into('<d', header, 171, z_offset)
    struct.pack_into('<d', header, 179, max_x)
    struct.pack_into('<d', header, 187, min_x)
    struct.pack_into('<d', header, 195, max_y)
    struct.pack_into('<d', header, 203, min_y)
    struct.pack_into('<d', header, 211, max_z)
    struct.pack_into('<d', header, 219, min_z)

    with open(path, 'wb') as f:
        f.write(header)
        for (x, y, z) in points:
            x_int = int(round((x - x_offset) / x_scale))
            y_int = int(round((y - y_offset) / y_scale))
            z_int = int(round((z - z_offset) / z_scale))
            record = struct.pack('<iiiHHB', x_int, y_int, z_int, 0, 0, 0)
            record += b'\0' * (POINT_DATA_RECORD_LENGTH - len(record))
            f.write(record)
    return n

def generate_points():
    """Generate two point clouds:
    1. Design surface: flat at BASE_Z
    2. Excavated: cone pit dug into the flat surface
    """
    random.seed(42)
    design_points = []
    excavated_points = []
    extent = RADIUS + 5
    x = CENTER_X - extent
    while x <= CENTER_X + extent:
        y = CENTER_Y - extent
        while y <= CENTER_Y + extent:
            dx = x - CENTER_X
            dy = y - CENTER_Y
            dist = math.sqrt(dx*dx + dy*dy)

            # Design surface: flat at BASE_Z
            design_z = BASE_Z + random.gauss(0, NOISE_STD)
            design_points.append((x, y, design_z))

            # Excavated surface: cone pit
            if dist <= RADIUS:
                # Inside the pit: z = BASE_Z - DEPTH * (1 - dist/RADIUS)
                # This creates an inverted cone — the pit gets deeper
                # toward the center.
                pit_depth = DEPTH * (1.0 - dist / RADIUS)
                excavated_z = BASE_Z - pit_depth
            else:
                excavated_z = BASE_Z
            excavated_z += random.gauss(0, NOISE_STD)
            excavated_points.append((x, y, excavated_z))

            y += SPACING
        x += SPACING
    return design_points, excavated_points

# ── Main ──
output_dir = sys.argv[1] if len(sys.argv) > 1 else './test-data'
os.makedirs(output_dir, exist_ok=True)

print("Generating synthetic pit excavation test data...")
print(f"  Pit radius: {RADIUS}m, depth: {DEPTH}m")
print(f"  Expected excavation volume: {EXPECTED_VOLUME:.2f} m³")

design_points, excavated_points = generate_points()

design_path = os.path.join(output_dir, 'design_surface.las')
excavated_path = os.path.join(output_dir, 'excavated_pit.las')

n1 = write_las(design_path, design_points)
n2 = write_las(excavated_path, excavated_points)

print(f"  Design surface: {design_path} ({n1} points)")
print(f"  Excavated pit:  {excavated_path} ({n2} points)")
print(f"  Analytical cut volume: {EXPECTED_VOLUME:.2f} m³")
print(f"  (cone: π × {RADIUS}² × {DEPTH} / 3 = {EXPECTED_VOLUME:.2f} m³)")
