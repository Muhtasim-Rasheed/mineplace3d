#version 330 core

in vec2 v_uv;
flat in int v_sprite_type;

out vec4 frag_color;

uniform sampler2D u_block_atlas;
// later:
// uniform sampler2D u_particle_atlas;

void main() {
    vec4 tex;

    if (v_sprite_type == 0) {
        tex = texture(u_block_atlas, v_uv);
    } else {
        // placeholder for future system
        tex = vec4(1.0, 0.0, 1.0, 1.0); // debug pink
    }

    if (tex.a < 0.1)
        discard;

    frag_color = tex;
}
