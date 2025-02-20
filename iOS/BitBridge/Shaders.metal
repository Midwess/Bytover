#include <metal_stdlib>
using namespace metal;

constant float PI = 3.1415926535897932384626433832795;

float3 hsv2rgb(float3 c) {
    float4 K = float4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    float3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

float4 circle(float2 uv, half4 mainColor, float hue, float2 center, float radius, float time) {
    float4 col = float4(0.0);
    float dist = length(uv - center); 
    float3 color = hsv2rgb(float3(mainColor.r, mainColor.g, mainColor.b));
    
    float2 deltaPos = uv - center;
    float angle = atan2(deltaPos.y, deltaPos.x);
    
    float movingHighlight = 0.5 + 0.5 * cos(angle - time * 0.5);
    movingHighlight = smoothstep(0.0, 1.0, movingHighlight);
    
    float outerGlowAngle = angle - time * 0.15;
    float outerHighlight = 0.5 + 0.5 * cos(outerGlowAngle);
    outerHighlight = smoothstep(0.0, 1.0, outerHighlight);
    
    float borderWidth = 0.018;
    float innerRadius = radius - borderWidth;
    float outerRadius = radius + borderWidth;
    float border = smoothstep(innerRadius, radius, dist) * 
                  (1.0 - smoothstep(radius, outerRadius, dist));
    
    float distFromBorder = max(dist - radius, 0.0);
    float glowWidth = 0.11;
    float baseGlow = exp(-1.5 * distFromBorder / glowWidth);
    
    float glowIntensity = baseGlow * mix(outerHighlight, movingHighlight, 
                         smoothstep(0.0, 0.2, distFromBorder));
    
    float intensity = (dist < radius) ? 0.0 :
                     (dist < outerRadius) ? 
                        mix(glowIntensity, border * (0.4 + 0.6 * movingHighlight), 
                            smoothstep(radius, outerRadius, dist)) :
                        glowIntensity;
    
    col.rgb = mix(float3(0.0), color, intensity);
    col.a = intensity;
    
    return col;
}

float2 drawCircle(int i) {
    return float2(0.5, 0.5);
}

[[ stitchable ]] half4 circleWave(float2 position, half4 color, float2 size, float time) {
    half2 uv = half2(position / size);
    float2 uvv = float2(uv.x, uv.y);
    float radius = 0.15;
    float noise = distance(uvv, drawCircle(0)) * 0.3;
    float hue = time * 0.1 + noise;

    float4 c = circle(uvv, color, hue, drawCircle(0), radius, time);
    
    return half4(c.r, c.g, c.b, c.a);
}
