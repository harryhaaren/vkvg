# Learning 2D Drawing with Vulkan (and WGPU)

This document explains the techniques used in the `vkvg` library to render high-quality 2D graphics with Vulkan. The goal is to provide a clear, step-by-step guide that can be adapted to other graphics APIs like `wgpu` in Rust.

We will explore the rendering process for various primitives, from simple lines to complex curves, and delve into advanced topics like text rendering and gradients. The explanations will include code examples and simplified mathematical concepts.

## Core Concepts

### Vulkan and WGPU

Vulkan is a low-level graphics and compute API that provides direct control over the GPU. `wgpu` is a higher-level graphics API that is designed to be safe and portable, with backends for Vulkan, Metal, DirectX, and OpenGL.

While `vkvg` uses Vulkan directly, the underlying concepts for 2D rendering are the same. By understanding how `vkvg` works, you can apply the same principles to your `wgpu` application in Rust. The main difference will be in the API calls and resource management, but the logic for tessellation, shaders, and antialiasing will be very similar.

### The Rendering Pipeline

In `vkvg`, the rendering process involves three main objects:

*   **`VkvgDevice`**: This represents the physical GPU and provides the connection to the Vulkan API. In `wgpu`, this is equivalent to the `wgpu::Device`.
*   **`VkvgSurface`**: This is the destination of the rendering, which can be a window or an off-screen image. In `wgpu`, this is the `wgpu::Surface`.
*   **`VkvgContext`**: This is the object that you use to issue drawing commands, such as `vkvg_line_to` or `vkvg_fill`. In `wgpu`, this is similar to the `wgpu::CommandEncoder`.

The general workflow is:

1.  Create a `VkvgDevice` and a `VkvgSurface`.
2.  Create a `VkvgContext` for the surface.
3.  Use the context to define paths and issue drawing commands.
4.  The context records these commands into a Vulkan command buffer.
5.  The command buffer is submitted to the GPU for execution.

This is very similar to how you would render with `wgpu`.

### Shaders

Shaders are small programs that run on the GPU. In `vkvg`, there are two main shaders:

*   **Vertex Shader (`vkvg_main.vert`)**: This shader is responsible for transforming the vertices of the shapes you want to draw from their original 2D coordinates into the normalized device coordinates that Vulkan expects. It also passes data like color and UV coordinates to the fragment shader.
*   **Fragment Shader (`vkvg_main.frag`)**: This shader is responsible for determining the color of each pixel (or "fragment") on the screen. It receives the data from the vertex shader and can perform more complex calculations, such as sampling textures for patterns or calculating gradients.

### Antialiasing (MSAA)

`vkvg` uses Multisample Antialiasing (MSAA) to create smooth, non-jagged lines. Here's a simplified explanation of how it works:

1.  **Multisample Texture:** Instead of a regular 2D texture, a multisample texture is used as the rendering target. This texture can store multiple color samples for each pixel.
2.  **Rasterization:** When a shape is rendered, the GPU determines which of these sample points are covered by the shape.
3.  **Shading:** The fragment shader is run once for each pixel, and the resulting color is written to all the covered sample points within that pixel.
4.  **Resolve:** Finally, the multiple samples for each pixel are averaged together to produce the final color. This averaging process is what creates the smooth edges.

In `vkvg`, the number of samples is configured when the `VkvgDevice` is created. In `wgpu`, you would configure this on your `wgpu::RenderPipeline`.

## Primitives

### Lines

Lines are the most basic primitive. In `vkvg`, you can draw a line like this:

```c
// From tests/lines.c
VkvgContext ctx = _initCtx();
vkvg_move_to(ctx, x1, y1);
vkvg_line_to(ctx, x2, y2);
vkvg_stroke(ctx);
vkvg_destroy(ctx);
```

The `vkvg_move_to` and `vkvg_line_to` functions define the path, and `vkvg_stroke` renders it.

#### The Magic Behind the Scenes: Tessellation

To draw a line with a certain thickness, `vkvg` doesn't just draw a single-pixel line. Instead, it creates a quadrilateral (a four-sided polygon, or "quad") that represents the line. This process is called **tessellation**.

Here's how it works for a line segment from P1 to P2 with a line width of `w`:

1.  **Find the Normal Vector:** A "normal" vector is a vector that is perpendicular to the line. If our line is represented by the vector `V = P2 - P1`, we can find a normal `N` by swapping the components of `V` and negating one of them.
    *   If `V = (dx, dy)`, then `N = (-dy, dx)`.

2.  **Normalize the Normal:** We scale the normal vector so that its length becomes 1. This is called "normalizing".
    *   `N_normalized = N / length(N)`

3.  **Extrude the Points:** We find the four corners of our quad by moving the start and end points of the line along the normalized normal vector. We move half the line width (`w/2`) in each direction.
    *   `A = P1 + (w/2) * N_normalized`
    *   `B = P2 + (w/2) * N_normalized`
    *   `C = P2 - (w/2) * N_normalized`
    *   `D = P1 - (w/2) * N_normalized`

These four points (A, B, C, D) form the quad that gets sent to the GPU for rendering.

#### What about Line Joins and Caps?

The same principle of tessellation applies to line joins (where two lines meet) and line caps (the ends of a line).

*   **Line Caps (`butt`, `round`, `square`):**
    *   `Butt`: The line ends exactly at the endpoint.
    *   `Square`: The line extends beyond the endpoint by half the line width.
    *   `Round`: A circular arc is added to the end of the line. This is done by generating a fan of triangles.
*   **Line Joins (`miter`, `round`, `bevel`):**
    *   `Miter`: The outer edges of the two lines are extended until they meet at a sharp point.
    *   `Round`: A circular arc is drawn to connect the two lines.
    *   `Bevel`: A triangular cap is added to connect the two lines.

#### In Rust and `wgpu`

In your `wgpu` application, you would implement a similar tessellation process in Rust. Your code would take a path as input and output a `Vec<Vertex>` and a `Vec<u16>` (for indices) that you can then use to create your vertex and index buffers.

Here's a conceptual example:

```rust
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

fn tessellate_line(p1: [f32; 2], p2: [f32; 2], width: f32) -> (Vec<Vertex>, Vec<u16>) {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];

    let length = (dx * dx + dy * dy).sqrt();
    let nx = -dy / length;
    let ny = dx / length;

    let half_w = width / 2.0;

    let a = [p1[0] + nx * half_w, p1[1] + ny * half_w];
    let b = [p2[0] + nx * half_w, p2[1] + ny * half_w];
    let c = [p2[0] - nx * half_w, p2[1] - ny * half_w];
    let d = [p1[0] - nx * half_w, p1[1] - ny * half_w];

    let vertices = vec![
        Vertex { position: a, color: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: b, color: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: c, color: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: d, color: [1.0, 0.0, 0.0, 1.0] },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}
```

### Rectangles (and Rounded Rectangles)

Drawing a rectangle is also straightforward. `vkvg` provides a `vkvg_rectangle` function:

```c
// from tests/rect_fill.c
VkvgContext ctx = _initCtx();
vkvg_rectangle(ctx, x, y, width, height);
vkvg_fill(ctx);
vkvg_destroy(ctx);
```

This creates a rectangular path, and `vkvg_fill` fills it with the current source (e.g., a solid color).

#### Tessellation

Tessellating a rectangle is even simpler than a line. A rectangle is already a quad, so we just need to generate the four vertices and the corresponding indices. The vertices are simply the four corners of the rectangle. The indices would be `[0, 1, 2, 0, 2, 3]` to form two triangles.

#### Rounded Rectangles

`vkvg` also supports rounded rectangles with the `vkvg_rounded_rectangle` function. This is a bit more complex. A rounded rectangle is composed of four straight line segments and four circular arcs.

The tessellation process for a rounded rectangle involves:

1.  **Generating the vertices for the four corner arcs.** This is done by approximating the arc with a series of small line segments, just like with circles (which we'll cover next).
2.  **Generating the vertices for the four straight line segments.**
3.  **Combining all the vertices and generating the indices** to create a single shape that can be filled.

#### In Rust and `wgpu`

For a simple rectangle, you can create a function that returns the vertices and indices for a quad. For a rounded rectangle, you would need to implement the logic to generate the corner arcs and the connecting line segments.

Here's a conceptual example for a simple rectangle:

```rust
fn tessellate_rectangle(x: f32, y: f32, width: f32, height: f32) -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        Vertex { position: [x, y], color: [0.0, 1.0, 0.0, 1.0] },
        Vertex { position: [x + width, y], color: [0.0, 1.0, 0.0, 1.0] },
        Vertex { position: [x + width, y + height], color: [0.0, 1.0, 0.0, 1.0] },
        Vertex { position: [x, y + height], color: [0.0, 1.0, 0.0, 1.0] },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}
```

### Circles and Arcs

Circles and arcs are drawn using the `vkvg_arc` function. A full circle is just an arc from 0 to 2*PI.

```c
// from tests/circles.c
VkvgContext ctx = _initCtx();
vkvg_arc(ctx, xc, yc, radius, 0, 2.0 * M_PI);
vkvg_fill(ctx);
vkvg_destroy(ctx);
```

#### Tessellation

A perfect circle cannot be represented with a finite number of triangles. Instead, we approximate the circle with a polygon that has many short sides.

The `_get_arc_step` function in `src/vkvg_context.c` calculates the angle step for the approximation based on the radius of the circle and the current transformation matrix. A larger radius will result in a smaller step size, and thus more vertices, to maintain a smooth appearance.

The vertices of the polygon are calculated using trigonometry:

*   `x = xc + radius * cos(angle)`
*   `y = yc + radius * sin(angle)`

We iterate from the start angle to the end angle with the calculated step, generating a vertex at each step. These vertices are then connected to form a fan of triangles.

#### In Rust and `wgpu`

You can implement a similar function in Rust that takes the center, radius, and angles as input and generates a `Vec<Vertex>` and `Vec<u16>`.

```rust
fn tessellate_arc(xc: f32, yc: f32, radius: f32, a1: f32, a2: f32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = vec![];
    let mut indices = vec![];

    let step = 0.1; // A smaller step gives a smoother circle
    let mut angle = a1;

    while angle < a2 {
        let x = xc + radius * angle.cos();
        let y = yc + radius * angle.sin();
        vertices.push(Vertex { position: [x, y], color: [0.0, 0.0, 1.0, 1.0] });
        angle += step;
    }
    // ... logic to create triangle fan indices ...
    (vertices, indices)
}
```

### Bézier Curves

Bézier curves are used to draw smooth, flowing lines. `vkvg` provides `vkvg_curve_to` for cubic Bézier curves and `vkvg_quadratic_to` for quadratic curves.

```c
// from tests/bezier.c
VkvgContext ctx = _initCtx();
vkvg_move_to(ctx, x0, y0);
vkvg_curve_to(ctx, x1, y1, x2, y2, x3, y3);
vkvg_stroke(ctx);
vkvg_destroy(ctx);
```

#### Tessellation

Like circles, Bézier curves are approximated with a series of short line segments. The `_recursive_bezier` function in `src/vkvg_context.c` uses a recursive subdivision algorithm to generate the points.

The algorithm works by repeatedly dividing the curve into two smaller Bézier curves. If a curve is "flat" enough (i.e., it can be approximated by a straight line within a certain tolerance), the algorithm stops subdividing and just adds the line segment. This is a very efficient way to generate a smooth-looking curve without using too many vertices.

The "flatness" is determined by calculating the distance of the control points from the line connecting the start and end points of the curve.

#### In Rust and `wgpu`

You can implement a similar recursive subdivision algorithm in Rust to tessellate Bézier curves. The core of the algorithm would be a function that takes the four control points of a cubic Bézier curve and recursively calls itself until the curve is flat enough.

## Advanced Topics

### Text Rendering (Signed Distance Fields)

`vkvg` uses a technique called Signed Distance Field (SDF) font rendering. This allows for high-quality text that can be scaled to any size without becoming pixelated.

Here's how it works:

1.  **Font Atlas:** The glyphs of the font are pre-rendered into a texture called a "font atlas". However, instead of storing the actual pixels of the glyph, we store the distance from each pixel to the nearest edge of the glyph. This is the "signed distance field". The distance is positive if the pixel is outside the glyph, and negative if it's inside.
2.  **Rendering:** When rendering a character, we just draw a quad that covers the area of the character.
3.  **Fragment Shader:** In the fragment shader, we sample the font atlas to get the signed distance for the current fragment. We can then use this distance to determine if the fragment is inside or outside the glyph. By checking if the distance is less than a certain threshold (e.g., 0.0), we can reconstruct the shape of the glyph.

The magic of SDF is that we can use the distance to create a smooth transition between the inside and outside of the glyph, which gives us perfectly antialiased text. The `smoothstep` function in GLSL is often used for this.

This is what the `inFontUV` varying and the `fontMap` sampler in `vkvg_main.frag` are used for.

### Gradients (Linear and Radial)

Gradients are implemented entirely in the fragment shader.

*   **Linear Gradients:**
    1.  The start and end points of the gradient are passed to the shader via a uniform buffer object (UBO).
    2.  For each fragment, the shader calculates its distance along the line defined by the start and end points.
    3.  This distance is then used to interpolate between the colors of the gradient stops (also defined in the UBO).

*   **Radial Gradients:**
    1.  The center points and radii of the two circles that define the gradient are passed to the shader.
    2.  For each fragment, the shader calculates its distance from the center of the inner circle.
    3.  This distance is then used to interpolate between the gradient colors.

The `smoothstep` function is used to create a smooth transition between the colors.
