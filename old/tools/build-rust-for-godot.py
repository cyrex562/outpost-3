#!/usr/bin/env python3
"""
Build Rust core library and copy it to Godot project bin directory.
Works across Windows, Linux, and macOS.
"""

import os
import sys
import platform
import shutil
import subprocess
from pathlib import Path


def get_library_filename():
    """Get the platform-specific library filename."""
    system = platform.system()

    if system == "Linux":
        return "liboutpost_3_core.so"
    elif system == "Darwin":  # macOS
        return "liboutpost_3_core.dylib"
    elif system == "Windows":
        return "outpost_3_core.dll"
    else:
        raise RuntimeError(f"Unsupported platform: {system}")


def main():
    # Get the project root directory (parent of tools directory)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    core_dir = project_root / "core"
    godot_bin_dir = project_root / "godot-project" / "bin"

    print(f"📦 Building Rust core library...")
    print(f"   Project root: {project_root}")
    print(f"   Core directory: {core_dir}")

    # Change to core directory and build
    os.chdir(core_dir)

    try:
        # Run cargo build with ffi feature
        result = subprocess.run(
            ["cargo", "build", "--features", "ffi"], check=True, capture_output=False
        )
    except subprocess.CalledProcessError as e:
        print(f"❌ Cargo build failed with exit code {e.returncode}")
        sys.exit(1)
    except FileNotFoundError:
        print("❌ Error: 'cargo' command not found. Is Rust installed?")
        sys.exit(1)

    # Create Godot bin directory if it doesn't exist
    godot_bin_dir.mkdir(parents=True, exist_ok=True)

    # Determine source and destination paths
    lib_filename = get_library_filename()
    source_path = project_root / "target" / "debug" / lib_filename
    dest_path = godot_bin_dir / lib_filename

    # Check if the library was built
    if not source_path.exists():
        print(f"❌ Error: Library not found at {source_path}")
        sys.exit(1)

    # Copy the library
    print(f"📋 Copying {lib_filename} to godot-project/bin/")
    try:
        shutil.copy2(source_path, dest_path)
        print(f"✅ Rust library copied to {dest_path}")
    except Exception as e:
        print(f"❌ Failed to copy library: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
