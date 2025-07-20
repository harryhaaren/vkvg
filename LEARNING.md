# Learning 2D Drawing with wgpu

This document explains how to use `wgpu` and Rust to achieve high-quality 2D drawing, inspired by the `vkvg` library. We will explore the rendering of various primitives, including lines, rectangles, circles, and bezier curves, with a focus on the underlying mathematics and implementation details.

## 1. Drawing Lines

Drawing a line might seem simple, but creating a high-quality, anti-aliased line requires some interesting techniques. In this section, we'll explore how `vkvg` and `wgpu` approach this problem.

### The `vkvg` Approach

The `vkvg` library uses a path-based API, similar to Cairo. To draw a line, you would typically:

1.  **Create a context:** `VkvgContext ctx = vkvg_create(surf);`
2.  **Set the starting point:** `vkvg_move_to(ctx, x1, y1);`
3.  **Define the end point:** `vkvg_line_to(ctx, x2, y2);`
4.  **Stroke the path:** `vkvg_stroke(ctx);`

This process creates a path in the context and then "strokes" it to generate the final line on the surface. The `vkvg` library handles the complex parts of this process, such as anti-aliasing, under the hood.

### The `wgpu` Approach: Tessellation

With `wgpu`, we need to be more explicit about how we draw a line. Since the GPU primarily deals with triangles, we need to convert our line into a set of triangles that can be rendered. This process is called **tessellation**.

A simple way to tessellate a line is to represent it as a thin rectangle (a quad). This quad is then broken down into two triangles.

Here's how we can do this in Rust with `wgpu`:

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

fn tessellate_line(p1: [f32; 2], p2: [f32; 2], width: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];

    let length = (dx * dx + dy * dy).sqrt();
    if length == 0.0 {
        return (vec![], vec![]);
    }
    let nx = -dy / length;
    let ny = dx / length;

    let half_w = width / 2.0;

    let a = [p1[0] + nx * half_w, p1[1] + ny * half_w];
    let b = [p2[0] + nx * half_w, p2[1] + ny * half_w];
    let c = [p2[0] - nx * half_w, p2[1] - ny * half_w];
    let d = [p1[0] - nx * half_w, p1[1] - ny * half_w];

    let vertices = vec![
        Vertex { position: a, color },
        Vertex { position: b, color },
        Vertex { position: c, color },
        Vertex { position: d, color },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}
```

#### The Math Explained

The core of this function is calculating the four corners of the rectangle that represents the line. To do this, we first need to find the **normal vector** of the line. The normal vector is a vector that is perpendicular to the line.

Given a line from `p1` to `p2`, the direction vector is `(dx, dy) = (p2.x - p1.x, p2.y - p1.y)`. A vector perpendicular to this is `(-dy, dx)`. We then **normalize** this vector by dividing it by its length, which gives us a unit vector `(nx, ny)`.

Once we have the normal vector, we can find the four corners of the rectangle by moving from the start and end points of the line in the direction of the normal vector by half of the desired line width.

This gives us the four vertices `a`, `b`, `c`, and `d` of our quad. We then create an index buffer that defines the two triangles that make up the quad.

### The Shader

The shader for drawing the line is very simple. The vertex shader passes the vertex position and color to the fragment shader, and the fragment shader simply outputs the color.

```wgsl
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
```

This is a basic implementation of line drawing. For anti-aliasing, we would need to use more advanced techniques, which we will explore in a later section.

## 2. Drawing Rectangles

Drawing a rectangle is very similar to drawing a line. We are still just drawing a quad, but this time the four corners are given directly.

### The `vkvg` Approach

In `vkvg`, you can draw a rectangle using the `vkvg_rectangle` function:

```c
vkvg_rectangle(ctx, x, y, width, height);
vkvg_fill(ctx); // Or vkvg_stroke(ctx);
```

This creates a rectangular path and then either fills it or strokes it.

### The `wgpu` Approach: Tessellation

For `wgpu`, we can create a function that generates the vertices and indices for a rectangle:

```rust
fn tessellate_rectangle(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        Vertex { position: [x, y], color },
        Vertex { position: [x + width, y], color },
        Vertex { position: [x + width, y + height], color },
        Vertex { position: [x, y + height], color },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    (vertices, indices)
}
```

This function is much simpler than the `tessellate_line` function because the vertices are already known. We just need to create the vertex and index buffers.

### Rounded Rectangles

Drawing a rounded rectangle is a bit more complex. A rounded rectangle is composed of four straight line segments and four circular arcs.

#### The `vkvg` Approach

`vkvg` provides a convenient function for this:

```c
vkvg_rounded_rectangle(ctx, x, y, width, height, radius);
vkvg_fill(ctx);
```

#### The `wgpu` Approach

In `wgpu`, we need to tessellate the rounded rectangle ourselves. This involves calculating the vertices for the four corner arcs and the four connecting rectangles. The arcs can be tessellated by dividing them into a series of small triangles.

Here's a simplified example of how you might tessellate a single rounded corner (the top-left corner):

```rust
fn tessellate_rounded_corner(
    center_x: f32,
    center_y: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: [f32; 4],
) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = vec![Vertex { position: [center_x, center_y], color }];
    let mut indices = vec![];
    let step = 0.1 / radius;
    let mut angle = start_angle;
    let mut i = 1;

    while angle < end_angle {
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        vertices.push(Vertex { position: [x, y], color });
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
        i += 1;
        angle += step;
    }
    // Connect the last vertex to the end angle
    let x = center_x + radius * end_angle.cos();
    let y = center_y + radius * end_angle.sin();
    vertices.push(Vertex { position: [x, y], color });
    indices.push(0);
    indices.push(i-1);
    indices.push(i);


    (vertices, indices)
}
```

To create a full rounded rectangle, you would call this function four times for each corner, and then create four rectangles to connect them. This is a more involved process, but it gives you full control over the rendering.

## 3. Drawing Circles and Arcs

Circles and arcs are fundamental shapes in 2D graphics. Let's see how they are handled in `vkvg` and `wgpu`.

### The `vkvg` Approach

`vkvg` provides functions for drawing arcs and circles:

```c
// Draw a full circle
vkvg_arc(ctx, center_x, center_y, radius, 0, 2 * M_PI);
vkvg_fill(ctx);

// Draw a partial arc
vkvg_arc(ctx, center_x, center_y, radius, start_angle, end_angle);
vkvg_stroke(ctx);
```

### The `wgpu` Approach: Tessellation

To draw a circle in `wgpu`, we can tessellate it into a fan of triangles. We place a vertex at the center of the circle and then create a series of vertices around the circumference. Each pair of vertices on the circumference forms a triangle with the center vertex.

```rust
fn tessellate_circle(xc: f32, yc: f32, radius: f32, color: [f32; 4]) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = vec![Vertex { position: [xc, yc], color }];
    let mut indices = vec![];
    let step = 0.1 / radius;
    let mut angle: f32 = 0.0;
    let mut i = 1;
    while angle < 2.0 * std::f32::consts::PI {
        let x = xc + radius * angle.cos();
        let y = yc + radius * angle.sin();
        vertices.push(Vertex { position: [x, y], color });
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
        i += 1;
        angle += step;
    }
    let len = indices.len();
    if len > 0 {
        indices[len - 1] = 1;
    }

    (vertices, indices)
}
```

Drawing an arc is very similar. The only difference is that you would not go all the way around the circle. You would start at `start_angle` and end at `end_angle`.

## 4. Drawing Bezier Curves

Bezier curves are used to draw smooth, flowing lines. They are defined by a set of control points. A cubic Bezier curve, which is commonly used in graphics, has a start point, an end point, and two control points that determine the shape of the curve.

### The `vkvg` Approach

`vkvg` provides functions for drawing Bezier curves:

```c
vkvg_move_to(ctx, start_x, start_y);
vkvg_curve_to(ctx, cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y);
vkvg_stroke(ctx);
```

### The `wgpu` Approach: Recursive Tessellation

Tessellating a Bezier curve is more complex than the other primitives. A common approach is to use recursive subdivision. The idea is to approximate the curve with a series of straight line segments.

The `recursive_bezier` function in the `wgpu_renderer` example demonstrates this technique. It recursively subdivides the curve until the resulting segments are flat enough to be approximated by a straight line. The flatness is determined by a tolerance value.

Once the curve is tessellated into a series of points, we can then use the `tessellate_line` function from the first section to draw the line segments that make up the curve.

```rust
fn recursive_bezier(
    x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32,
    vertices: &mut Vec<Vertex>, color: [f32; 4]
) {
    let tolerance = 0.1;
    let x12 = (x1 + x2) / 2.0;
    let y12 = (y1 + y2) / 2.0;
    let x23 = (x2 + x3) / 2.0;
    let y23 = (y2 + y3) / 2.0;
    let x34 = (x3 + x4) / 2.0;
    let y34 = (y3 + y4) / 2.0;
    let x123 = (x12 + x23) / 2.0;
    let y123 = (y12 + y23) / 2.0;
    let x234 = (x23 + x34) / 2.0;
    let y234 = (y23 + y34) / 2.0;
    let x1234 = (x123 + x234) / 2.0;
    let y1234 = (y123 + y234) / 2.0;

    let dx = x4 - x1;
    let dy = y4 - y1;
    let d2 = ((x2 - x4) * dy - (y2 - y4) * dx).abs();
    let d3 = ((x3 - x4) * dy - (y3 - y4) * dx).abs();

    if (d2 + d3) * (d2 + d3) < tolerance * (dx * dx + dy * dy) {
        vertices.push(Vertex { position: [x4, y4], color });
        return;
    }

    recursive_bezier(x1, y1, x12, y12, x123, y123, x1234, y1234, vertices, color);
    recursive_bezier(x1234, y1234, x234, y234, x34, y34, x4, y4, vertices, color);
}

fn tessellate_bezier(
    x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32,
    width: f32, color: [f32; 4]
) -> (Vec<Vertex>, Vec<u16>) {
    let mut points = vec![];
    recursive_bezier(x1, y1, x2, y2, x3, y3, x4, y4, &mut points, color);

    let mut vertices = vec![];
    let mut indices = vec![];
    for i in 0..points.len() - 1 {
        let p1 = points[i].position;
        let p2 = points[i + 1].position;
        let (v, mut ind) = tessellate_line(p1, p2, width, color);
        for i in &mut ind {
            *i += vertices.len() as u16;
        }
        vertices.extend(v);
        indices.extend(ind);
    }
    (vertices, indices)
}
```

This covers the basic primitives. In the next section, we will discuss anti-aliasing.

## 5. Anti-Aliasing

Anti-aliasing is a technique used to smooth out the jagged edges of rendered shapes. This is a crucial step for achieving high-quality 2D graphics.

### The `vkvg` Approach: Multisample Anti-Aliasing (MSAA)

The `vkvg` library uses a common technique called Multisample Anti-Aliasing (MSAA). MSAA works by taking multiple samples per pixel and then averaging the results to produce the final pixel color. This is a hardware-supported feature in modern GPUs and is relatively easy to enable.

In `vkvg`, you can enable MSAA when you create the `VkvgDevice`:

```c
VkvgDevice dev = vkvg_device_create_multisample(vk_inst, vk_phy, vk_dev, q_fam, q_idx, VK_SAMPLE_COUNT_4_BIT);
```

This creates a device that will use 4 samples per pixel for rendering.

### The `wgpu` Approach

`wgpu` also supports MSAA. You can enable it by setting the `sample_count` in the `wgpu::RenderPipelineDescriptor` and the `wgpu::TextureDescriptor`.

However, for 2D vector graphics, there is another technique that can produce even better results: **Analytic Anti-Aliasing**. This technique involves calculating the exact coverage of a shape for each pixel and then using that information to blend the color of the shape with the background color.

This is a more advanced technique that requires more complex shaders. The basic idea is to calculate the signed distance from a fragment to the edge of the shape. This distance can then be used to determine how much the fragment is covered by the shape.

For example, to draw an anti-aliased line, you could pass the line's endpoints and width to the fragment shader. The shader would then calculate the distance from the current fragment to the line and use that distance to calculate the alpha value of the fragment. Fragments that are far from the line would have an alpha of 0, fragments that are close to the line would have an alpha of 1, and fragments that are on the edge of the line would have an alpha between 0 and 1.

This technique is more computationally expensive than MSAA, but it can produce very high-quality results.

This concludes our tour of 2D drawing with `wgpu`. We have covered the basics of drawing lines, rectangles, circles, and Bezier curves, and we have touched on the important topic of anti-aliasing. With this knowledge, you should be well-equipped to start building your own 2D rendering engine in Rust and `wgpu`.
