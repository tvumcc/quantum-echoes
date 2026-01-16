# Quantum Echoes: Interactive Quantum Mechanics

An interactive simulator for the time-dependent Schrödinger Equation. Written in Rust and Vulkan using [vulkano](https://github.com/vulkano-rs/vulkano).

## Features
- A brush tool to set initial conditions and regions of potential energy
- Creating particles as Gaussian wave packets with an initial momentum vector
- Four visibility layers: Wave Function, Probability, and Real and Imaginary components
- Three boundary condition options: Dirichlet, Neumann, and Periodic
- Leapfrog integration as described in [this paper](https://pubs.aip.org/aip/cip/article/5/6/596/279764/A-fast-explicit-algorithm-for-the-time-dependent)

## Installation
Windows and Linux builds are available on the [Releases]() tab.