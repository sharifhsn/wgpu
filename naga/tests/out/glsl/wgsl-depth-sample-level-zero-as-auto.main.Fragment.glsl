#version 460 core
uniform sampler2DShadow _group_0_binding_0_fs;

layout(location = 0) out float _fs2p_location0;

void main() {
    float _e5 = texture(_group_0_binding_0_fs, vec3(vec2(0.5), 0.5));
    _fs2p_location0 = _e5;
    return;
}

