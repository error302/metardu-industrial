#!/usr/bin/env python3
"""
Generate a realistic synthetic stockpile LAS file for testing.

Creates a cone-shaped stockpile on flat ground — the classic mining
volume calculation scenario. The analytical volume of a cone is
V = π r² h / 3, so we can cross-validate the app's calculation
against this known value.

Usage:
  python3 generate_stockpile.py [output_path]

Output: stockpile.las (default: ./test-data/stockpile.las)
"""

import struct
import math
import random
import os
import sys

# ── Stockpile parameters ──
RADIUS = 20.0      # meters
HEIGHT = 8.0       # meters above base
BASE_Z = 100.0     # base elevation (flat ground)
CENTER_X = 500.0   # UTM easting
CENTER_Y = 5000.0  # UTM northing
SPACING = 0.5      # meters between points (drone survey density)
NOISE_STD = 0.02   # ±2cm noise (realistic GPS+lidar)
# Expected cone volume = π * R² * H / 3
EXPECTED_VOLUME = math.pi * RADIUS**2 * HEIGHT / 3

# ── LAS header (LAS 1.2, PDRF 0) ──
HEADER_SIZE = 227
POINT_DATA_RECORD_LENGTH = 20  # PDRF 0: 20 bytes per point

def write_las(path, points):
    """Write a LAS 1.2 file with the given points."""
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
    struct.pack_into('<H', header, 4, 0)
    struct.pack_into('<H', header, 6, 0)
    header[26:58] = b'MetaRDU Synthetic Test'.ljust(32, b'\0')
    header[58:90] = b'generate_stockpile.py'.ljust(32, b'\0')
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

def generate_stockpile_points():
    random.seed(42)
    points = []
    extent = RADIUS + 5
    x = CENTER_X - extent
    while x <= CENTER_X + extent:
        y = CENTER_Y - extent
        while y <= CENTER_Y + extent:
            dx = x - CENTER_X
            dy = y - CENTER_Y
            dist = math.sqrt(dx*dx + dy*dy)
            if dist <= RADIUS:
                z = BASE_Z + HEIGHT * (1.0 - dist / RADIUS)
            else:
                z = BASE_Z
            z += random.gauss(0, NOISE_STD)
            points.append((x, y, z))
            y += SPACING
        x += SPACING
    return points

# ── Main ──
output_path = sys.argv[1] if len(sys.argv) > 1 else './test-data/stockpile.las'
os.makedirs(os.path.dirname(output_path) or '.', exist_ok=True)

print(f"Generating synthetic stockpile LAS file...")
print(f"  Radius: {RADIUS}m, Height: {HEIGHT}m, Base: {BASE_Z}m")
print(f"  Expected cone volume: {EXPECTED_VOLUME:.2f} m³")

points = generate_stockpile_points()
n = write_las(output_path, points)
file_size = os.path.getsize(output_path)
print(f"  Written to {output_path} ({file_size} bytes, {n} points)")
print(f"  Analytical volume: {EXPECTED_VOLUME:.2f} m³")
