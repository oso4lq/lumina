struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(2) var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOut {
    // quad в координатах 0..1 (triangle-strip из 4 вершин)
    var quad = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    let p = quad[vid];
    var out: VertexOut;
    out.pos = transform * vec4<f32>(p, 0.0, 1.0);
    out.uv = p; // uv совпадает с позицией quad в 0..1
    return out;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
