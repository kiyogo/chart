#if NEW_SHADER_INTERFACE
    #define I in
    #define O out
    #define V(x) x
#else
    #define I attribute
    #define O varying
    #define V(x) vec3(x)
#endif

#ifdef GL_ES
    precision mediump float;
#endif

uniform vec2 u_screen_size;
uniform float u_pixels_per_point;
uniform int u_is_font;

I vec2 a_pos;
I vec4 a_srgba; // 0-255 sRGB
I vec2 a_tc;
O vec4 v_rgba_in_gamma;
O vec2 v_tc;

void main() {
    vec2 pos = a_pos;
    if (u_is_font != 0 && u_pixels_per_point > 0.0) {
        // High-precision GPU hardware pixel-grid snapping:
        // Culls fractional sub-pixel offsets, aligning glyph stems directly to physical monitor pixels.
        pos = floor(a_pos * u_pixels_per_point + 0.5) / u_pixels_per_point;
    }
    gl_Position = vec4(
                      2.0 * pos.x / u_screen_size.x - 1.0,
                      1.0 - 2.0 * pos.y / u_screen_size.y,
                      0.0,
                      1.0);
    v_rgba_in_gamma = a_srgba / 255.0;
    v_tc = a_tc;
}
