# Marching Cubes

An interactive 3D visualization of the [Marching Cubes](https://en.wikipedia.org/wiki/Marching_cubes) algorithm, built with Rust and the [Bevy](https://bevyengine.org/) game engine for [my YouTube video](https://www.youtube.com/watch?v=OrBzjwW2OnU).

The scene displays a 3D grid of nodes. Each node holds a density value, and the algorithm continuously generates a smooth isosurface mesh at the boundary between positive and negative density values. Clicking nodes and changing the grid resolution lets you see in real time how the algorithm responds to different inputs.

## Features

- Click any node to toggle its density, instantly rebuilding the isosurface mesh
- Increase or decrease the grid resolution to explore different voxel configurations
- Orbiting camera so you can inspect the mesh from any angle
- Full 256-case Marching Cubes lookup table (Lorensen-Cline) with linear interpolation and flat normals

## Controls

| Key / Input     | Action                          |
|-----------------|---------------------------------|
| Click a node    | Toggle density (on/off)         |
| Up Arrow        | Increase grid resolution (max 8)|
| Down Arrow      | Decrease grid resolution (min 1)|
| Left Arrow      | Orbit camera left               |
| Right Arrow     | Orbit camera right              |

## Running

This project requires [Rust](https://www.rust-lang.org/tools/install) and Cargo.

```sh
# Debug build (faster compile time)
cargo run

# Release build (don't know why you'd want this, but here it is)
cargo run --release
```
