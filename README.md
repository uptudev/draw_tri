# draw_tri

This project uses the wgpu crate to draw a triangle with basic UV shading to the screen. It is compilable to WASM to be run in a canvas.

## Getting Started

To get started, clone the repository and install the dependencies:
```
git clone https://github.com/[your-username]/rust-triangle.git
cd rust-triangle
cargo install --dependencies web
```

## Running the Project

To run the project, run the following command:
```
cargo run --target=wasm32-unknown-unknown
```

This will open a web browser window with the triangle rendered in a canvas.

## Features

This project demonstrates the following features:

* Using the `wgpu` crate to draw a triangle
* Applying basic UV shading to a triangle
* Compiling a Rust project to WASM to be run in a canvas

## Further Reading

* [The `wgpu` crate documentation](https://docs.rs/wgpu/)
* [The WebGPU specification](https://www.khronos.org/webgpu/)
