#include <metal_stdlib>
using namespace metal;

constant float PI = 3.1415926535897932384626433832795;

float4 circle(float2 uv, float3 mainColor, float3 subColor, float2 center, float sharpness, float radius, float time) {
    float dist = length(uv - center);
    float4 col = float4(0.0);
    
    // Calculate angle for rotation
    float2 deltaPos = uv - center;
    float angle = atan2(deltaPos.y, deltaPos.x);
    
    // Create rotating highlight
    float rotationSpeed = 2.0;
    float rotatingGlow = 0.5 * (1.0 + sin(angle + time * rotationSpeed));
    
    // Border parameters
    float borderWidth = 0.006;
    float innerRadius = radius - borderWidth;
    float outerRadius = radius + borderWidth;
    float border = smoothstep(innerRadius, radius, dist) * 
                  (1.0 - smoothstep(radius, outerRadius, dist));
    
    border *= sharpness;
    
    float distFromBorder = abs(dist - radius);
    
    // Glow parameters
    float glowWidth = 0.22;
    float baseGlow = exp(-4.0 * distFromBorder / glowWidth);
    
    // Combine rotating glow
    float glowIntensity = baseGlow * (0.2 + 0.6 * rotatingGlow);
    
    float innerFill = 1.0 - smoothstep(radius * 0.95, radius, dist);
    
    float intensity = max(glowIntensity, border);
    intensity = max(intensity, innerFill * 0.3);
    
    if (intensity > 0.001) {
        float3 glowColor = mix(mainColor * 0.8, subColor, rotatingGlow);
        col.rgb = mix(float3(0.0), glowColor, intensity);
        col.a = intensity;
    }
    
    return col;
}

float2 drawCircle(int i) {
    return float2(0.5, 0.5); // Center point
}

[[ stitchable ]] half4 circleWave(float2 position, half4 inputColor, float2 size, half4 subColor, float time) {
    half2 uv = half2(position / size);
    float2 uvv = float2(uv.x, uv.y);
    float3 color = float3(inputColor.r, inputColor.g, inputColor.b);
    float3 color2 = float3(subColor.r, subColor.g, subColor.b);
    
    float4 c = circle(uvv, color, color2, drawCircle(0), 0.15, 0.15, sin((time))) +
               circle(uvv, color2, color, drawCircle(0), 2.5, 0.13, sin((time + PI * 0.5)));

    return half4(c.r, c.g, c.b, c.a);
}
