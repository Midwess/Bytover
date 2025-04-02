#include <metal_stdlib>
using namespace metal;

constant float PI = 3.1415926535897932384626433832795;

float4 circle(float2 uv, float3 mainColor, float3 subColor, float2 center, float sharpness, float radius, float time) {
    float dist = length(uv - center);
    
    float2 deltaPos = uv - center;
    float angle = atan2(deltaPos.y, deltaPos.x);
    float rotatingGlow = 0.3 * (1.0 + sin(angle + time * 2.0));
    
    float borderWidth = 0.006;
    float border = smoothstep(radius - borderWidth, radius, dist) * 
                  (1.0 - smoothstep(radius, radius + borderWidth, dist)) * sharpness;
    
    float distFromBorder = abs(dist - radius);
    float glowIntensity = exp(-3.0 * distFromBorder / 0.2) * (0.3 + 0.8 * rotatingGlow);
    
    float innerFill = 1.0 - smoothstep(radius * 0.95, radius, dist);
    
    float intensity = max(max(glowIntensity, border), innerFill * 0.3);
    
    float3 glowColor = mix(mainColor * 0.8, subColor, rotatingGlow);
    return float4(mix(float3(0.0), glowColor, intensity), intensity);
}

float2 drawCircle(int i) {
    return float2(0.5, 0.5);
}

[[ stitchable ]] half4 circleWave(float2 position, half4 inputColor, float2 size, half4 subColor, float time) {
    float2 uvv = float2(position / size);
    float3 color = float3(inputColor.rgb);
    float3 color2 = float3(subColor.rgb);
    
    float4 c = circle(uvv, color, color2, drawCircle(0), 0.5, 0.18, sin(time)) +
               circle(uvv, color2, color, drawCircle(0), 2.0, 0.13, sin(time + PI * 0.5)) * 1.8;

    return half4(c.r, c.g, c.b, c.a);
}

