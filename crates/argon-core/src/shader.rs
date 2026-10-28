//! Shadertoy-style GLSL fragment shaders (FR-G6, FR-G7, D6).
//!
//! The user writes `void mainImage(out vec4 fragColor, in vec2 fragCoord)`
//! and may use `iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iFrameRate`,
//! `iMouse`, `iDate`, `iChannel0..3`, `iChannelResolution`, plus their own
//! `uniform float x;` declarations. We wrap that in a prelude that declares
//! the bindings Argon provides, parse and validate it with naga, and report
//! errors with the user's line numbers.

use std::sync::Arc;

/// Maximum number of user `uniform` declarations per shader.
pub const MAX_USER_UNIFORMS: usize = 16;
pub const CHANNELS: usize = 4;

/// Globals block layout (std140), 144 bytes. Kept in sync with PRELUDE.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShaderGlobals {
    /// iResolution.xyz (z = pixel aspect, 1)
    pub resolution: [f32; 3],
    pub time: f32,
    pub time_delta: f32,
    pub frame: i32,
    pub frame_rate: f32,
    pub sample_rate: f32,
    /// Shadertoy iMouse: xy current (y up), zw click position, sign = button.
    pub mouse: [f32; 4],
    pub date: [f32; 4],
    /// Pixel offset of the drawn rect inside the target (xy), unused zw.
    pub offset: [f32; 4],
    /// iChannelResolution[4], each vec3 padded to vec4.
    pub channel_resolution: [[f32; 4]; CHANNELS],
}

unsafe impl bytemuck_lite::Pod for ShaderGlobals {}

/// Minimal stand-in so core does not depend on bytemuck: we only need the
/// byte view of a `#[repr(C)]` plain-data struct.
pub mod bytemuck_lite {
    /// # Safety
    /// Implement only for `#[repr(C)]` types made of plain numeric fields
    /// with no padding that could hold uninitialized bytes being read.
    pub unsafe trait Pod: Copy {}
    pub fn bytes_of<T: Pod>(v: &T) -> &[u8] {
        unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UniformType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    Bool,
}

impl UniformType {
    fn parse(s: &str) -> Option<UniformType> {
        match s {
            "float" => Some(UniformType::Float),
            "vec2" => Some(UniformType::Vec2),
            "vec3" => Some(UniformType::Vec3),
            "vec4" => Some(UniformType::Vec4),
            "int" => Some(UniformType::Int),
            "bool" => Some(UniformType::Bool),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserUniform {
    pub name: String,
    pub ty: UniformType,
    /// Index into the `_user[]` vec4 array.
    pub slot: usize,
}

/// A parsed, validated shader ready for the renderer.
pub struct CompiledShader {
    pub module: naga::Module,
    pub info: naga::valid::ModuleInfo,
    pub uniforms: Vec<UserUniform>,
    /// Full GLSL after preprocessing (for debugging).
    pub source: String,
}

impl PartialEq for CompiledShader {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl std::fmt::Debug for CompiledShader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompiledShader({} uniforms)", self.uniforms.len())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderError {
    /// 1-based line in the user's source, if known.
    pub line: Option<u32>,
    pub message: String,
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "shader line {l}: {}", self.message),
            None => write!(f, "shader: {}", self.message),
        }
    }
}

const PRELUDE: &str = r#"#version 450
layout(location = 0) in vec2 argonUv;
layout(location = 1) in vec4 argonColor;
layout(location = 0) out vec4 argonOut;
layout(set = 0, binding = 0) uniform ArgonGlobals {
    vec3 iResolution;
    float iTime;
    float iTimeDelta;
    int iFrame;
    float iFrameRate;
    float iSampleRate;
    vec4 iMouse;
    vec4 iDate;
    vec4 iOffset;
    vec4 _iChannelRes[4];
};
layout(set = 0, binding = 1) uniform ArgonUser { vec4 _user[16]; };
layout(set = 0, binding = 2) uniform texture2D _argonTex0;
layout(set = 0, binding = 3) uniform sampler _argonSamp0;
layout(set = 0, binding = 4) uniform texture2D _argonTex1;
layout(set = 0, binding = 5) uniform sampler _argonSamp1;
layout(set = 0, binding = 6) uniform texture2D _argonTex2;
layout(set = 0, binding = 7) uniform sampler _argonSamp2;
layout(set = 0, binding = 8) uniform texture2D _argonTex3;
layout(set = 0, binding = 9) uniform sampler _argonSamp3;
#define iChannel0 sampler2D(_argonTex0, _argonSamp0)
#define iChannel1 sampler2D(_argonTex1, _argonSamp1)
#define iChannel2 sampler2D(_argonTex2, _argonSamp2)
#define iChannel3 sampler2D(_argonTex3, _argonSamp3)
#define iChannelResolution(i) (_iChannelRes[i].xyz)
#define vUv argonUv
#define vColor argonColor
// Shadertoy samples textures bottom-up (v = 0 is the bottom row); Argon
// images are top-down. Flip v in the sampling functions so a shader that
// reads a channel at fragCoord / iResolution shows the image upright.
#define texture(s, uv) texture(s, vec2((uv).x, 1.0 - (uv).y))
#define textureLod(s, uv, l) textureLod(s, vec2((uv).x, 1.0 - (uv).y), l)
#define texelFetch(s, p, l) texelFetch(s, ivec2((p).x, textureSize(s, 0).y - 1 - (p).y), l)
// Argon-native sampling with top-down coordinates (what vUv uses).
#define textureTopDown(s, uv) texture(s, vec2((uv).x, 1.0 - (uv).y))
"#;

const POSTLUDE: &str = r#"
void main() {
    vec2 argon_fc = vec2(gl_FragCoord.x - iOffset.x, iResolution.y - (gl_FragCoord.y - iOffset.y));
    vec4 argon_col = vec4(0.0, 0.0, 0.0, 1.0);
    mainImage(argon_col, argon_fc);
    argonOut = argon_col;
}
"#;

/// Hoist `uniform TYPE name;` declarations out of the user source into
/// `#define`s onto the `_user` array. Lines are replaced by blank lines so
/// line numbers stay meaningful.
fn preprocess(user: &str) -> Result<(String, String, Vec<UserUniform>), ShaderError> {
    let mut defines = String::new();
    let mut body = String::with_capacity(user.len());
    let mut uniforms = Vec::new();
    for (i, line) in user.lines().enumerate() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("uniform ") {
            let rest = rest.trim_end_matches(';').trim();
            let mut parts = rest.split_whitespace();
            let (ty, name) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            let Some(ty) = UniformType::parse(ty) else {
                return Err(ShaderError { line: Some(i as u32 + 1), message: format!("unsupported uniform type in '{t}' (use float, vec2, vec3, vec4, int or bool)") });
            };
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') || name.starts_with("argon") || name.starts_with('i') && name.len() > 1 && name.chars().nth(1).map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
                return Err(ShaderError { line: Some(i as u32 + 1), message: format!("bad uniform declaration '{t}'") });
            }
            if uniforms.len() >= MAX_USER_UNIFORMS {
                return Err(ShaderError { line: Some(i as u32 + 1), message: format!("too many uniforms (max {MAX_USER_UNIFORMS})") });
            }
            let slot = uniforms.len();
            let expr = match ty {
                UniformType::Float => format!("(_user[{slot}].x)"),
                UniformType::Vec2 => format!("(_user[{slot}].xy)"),
                UniformType::Vec3 => format!("(_user[{slot}].xyz)"),
                UniformType::Vec4 => format!("(_user[{slot}])"),
                UniformType::Int => format!("(int(_user[{slot}].x))"),
                UniformType::Bool => format!("(_user[{slot}].x != 0.0)"),
            };
            defines.push_str(&format!("#define {name} {expr}\n"));
            uniforms.push(UserUniform { name: name.to_string(), ty, slot });
            body.push('\n');
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    Ok((defines, body, uniforms))
}

/// The vertex stage paired with user fragment shaders. Written in GLSL so
/// naga gives both stages matching interpolation qualifiers.
pub const VERTEX_GLSL: &str = r#"#version 450
layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;
layout(location = 0) out vec2 argonUv;
layout(location = 1) out vec4 argonColor;
void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
    argonUv = uv;
    argonColor = color;
}
"#;

pub fn vertex_module() -> naga::Module {
    let mut frontend = naga::front::glsl::Frontend::default();
    frontend.parse(&naga::front::glsl::Options::from(naga::ShaderStage::Vertex), VERTEX_GLSL).expect("built-in vertex shader")
}

/// Compile a Shadertoy-style fragment shader.
pub fn compile(user: &str) -> Result<Arc<CompiledShader>, ShaderError> {
    if !user.contains("mainImage") {
        return Err(ShaderError { line: None, message: "shader must define void mainImage(out vec4 fragColor, in vec2 fragCoord)".to_string() });
    }
    let (defines, body, uniforms) = preprocess(user)?;
    let prelude_lines = PRELUDE.lines().count() + defines.lines().count();
    let source = format!("{PRELUDE}{defines}{body}{POSTLUDE}");

    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options::from(naga::ShaderStage::Fragment);
    let module = frontend.parse(&options, &source).map_err(|errs| {
        let first = errs.errors.first();
        let line = first.and_then(|e| e.meta.to_range()).map(|r| line_of(&source, r.start));
        let message = first.map(|e| format!("{:?}", e.kind)).unwrap_or_else(|| "parse error".to_string());
        ShaderError { line: user_line(line, prelude_lines), message: clean_message(&message) }
    })?;
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .map_err(|e| {
            let line = e.spans().next().and_then(|(span, _)| span.to_range()).map(|r| line_of(&source, r.start));
            ShaderError { line: user_line(line, prelude_lines), message: clean_message(&e.as_inner().to_string()) }
        })?;
    Ok(Arc::new(CompiledShader { module, info, uniforms, source }))
}

fn line_of(src: &str, byte: usize) -> u32 {
    src[..byte.min(src.len())].matches('\n').count() as u32 + 1
}

fn user_line(line: Option<u32>, prelude_lines: usize) -> Option<u32> {
    line.and_then(|l| (l as usize).checked_sub(prelude_lines)).map(|l| l as u32).filter(|l| *l >= 1)
}

fn clean_message(m: &str) -> String {
    // naga's Debug output for error kinds is verbose; keep it readable.
    let m = m.replace("SemanticError(", "").replace("InvalidToken(", "unexpected token ").replace("UnknownVariable(", "unknown name ");
    m.trim_end_matches(')').replace('"', "'").to_string()
}

/// Shader sources available via `shader_source(name)`.
pub static BUILTIN_SHADERS: &[(&str, &str)] = &[
    (
        "plasma",
        r#"// plasma: the classic sine-sum, in Grid colors.
uniform float speed;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float t = iTime * (speed == 0.0 ? 1.0 : speed);
    float v = sin(uv.x * 10.0 + t) + sin((uv.y * 10.0 + t) * 0.7)
            + sin((uv.x + uv.y) * 8.0 + t * 1.3) + sin(length(uv - 0.5) * 20.0 - t * 2.0);
    v = v * 0.25 + 0.5;
    vec3 col = mix(vec3(0.0, 0.05, 0.12), vec3(0.0, 0.85, 1.0), v);
    col = mix(col, vec3(1.0, 0.5, 0.1), smoothstep(0.85, 1.0, v));
    fragColor = vec4(col, 1.0);
}
"#,
    ),
    (
        "ripple",
        r#"// ripple: distorts iChannel0 with rings spreading from the mouse.
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec2 m = iMouse.xy / iResolution.xy;
    float d = distance(uv, m);
    float wave = sin(d * 60.0 - iTime * 6.0) * 0.01 / (1.0 + d * 8.0);
    fragColor = texture(iChannel0, uv + normalize(uv - m + 1e-4) * wave);
}
"#,
    ),
    (
        "pixelate",
        r#"// pixelate: iChannel0 in big pixels. uniform size = pixel size.
uniform float size;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    float s = size <= 0.0 ? 8.0 : size;
    vec2 p = floor(fragCoord / s) * s + s * 0.5;
    fragColor = texture(iChannel0, p / iResolution.xy);
}
"#,
    ),
    (
        "scanlines",
        r#"// scanlines: iChannel0 with CRT-style lines and vignette.
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec4 c = texture(iChannel0, uv);
    float line = 0.85 + 0.15 * sin(fragCoord.y * 3.14159);
    float vig = 1.0 - 0.6 * pow(length(uv - 0.5), 2.0);
    fragColor = vec4(c.rgb * line * vig, c.a);
}
"#,
    ),
    (
        "tint",
        r#"// tint: multiply iChannel0 by a color and the vertex color (sprite mode).
uniform vec3 color;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec4 c = textureTopDown(iChannel0, vUv) * vColor;
    vec3 k = color == vec3(0.0) ? vec3(1.0) : color;
    fragColor = vec4(c.rgb * k, c.a);
}
"#,
    ),
];

pub fn builtin_shader(name: &str) -> Option<&'static str> {
    BUILTIN_SHADERS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, s)| *s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_shadertoy_style_source() {
        let src = r#"
uniform float speed;
uniform vec3 tintc;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec3 col = 0.5 + 0.5 * cos(iTime * speed + uv.xyx + vec3(0, 2, 4));
    col *= tintc;
    col += texture(iChannel0, uv).rgb * 0.1;
    fragColor = vec4(col, 1.0);
}"#;
        let sh = compile(src).unwrap();
        assert_eq!(sh.uniforms.len(), 2);
        assert_eq!(sh.uniforms[0].name, "speed");
        assert_eq!(sh.uniforms[1].ty, UniformType::Vec3);
    }

    #[test]
    fn errors_point_at_user_lines() {
        let src = "void mainImage(out vec4 fragColor, in vec2 fragCoord) {\n    vec3 c = badName * 2.0;\n    fragColor = vec4(c, 1.0);\n}";
        let e = compile(src).unwrap_err();
        assert_eq!(e.line, Some(2), "{e:?}");
        assert!(e.message.to_lowercase().contains("badname"), "{e:?}");
        let e2 = compile("void main() {}").unwrap_err();
        assert!(e2.message.contains("mainImage"));
    }

    #[test]
    fn builtins_compile() {
        for (name, src) in BUILTIN_SHADERS {
            compile(src).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn globals_layout_is_160_bytes() {
        assert_eq!(std::mem::size_of::<ShaderGlobals>(), 144);
    }
}
