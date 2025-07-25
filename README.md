# escpos-embed-image

A procedural macro for embedding monochrome, dithered images into ESC/POS printer drivers at compile time.

Designed as a companion to `escpos-embedded`, this crate processes PNG images at compile time and generates a static `Image<'static>` compatible with the `no_std`, allocation-free printer interface.

## Features

- Compile-time image loading and conversion
- Converts PNG to 1-bit dithered format (Bi-level Floyd-Steinberg)
- Outputs `Image<'static>` struct ready for printing
- No runtime dependencies

## Example

```rust
use escpos_embedded::Image;
use escpos_embed_image::embed_image;

static LOGO: Image<'static> = embed_image!("assets/logo.png");
```

## How it works

- At compile time, the macro loads the PNG file
- Converts it to grayscale, then applies dithering
- Packs the result into 1-bit-per-pixel (row-major) format
- Emits a static `Image` instance with dimensions and data

## Requirements

- Input must be a valid `.png` file path
- The output image must be small enough to fit in flash/ROM (no heap)

## Crate Structure

This is a separate crate from `escpos-embedded` because it uses `proc-macro` and requires `std`. It is intended for use on the host during build/compile time only.
