#include <metal_stdlib>
using namespace metal;

constant float PI = 3.1415926535897932384626433832795;

// Circle shader functions
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
    float glowIntensity = baseGlow * (0.3 + 0.8 * rotatingGlow);
    
    float innerFill = 1.0 - smoothstep(radius * 0.95, radius, dist);
    
    float intensity = max(glowIntensity, border);
    intensity = max(intensity, innerFill * 0.3);
    
    float3 glowColor = mix(mainColor * 0.8, subColor, rotatingGlow);
    col.rgb = mix(float3(0.0), glowColor, intensity);
    col.a = intensity;
    
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
    
    float4 c = circle(uvv, color, color2, drawCircle(0), 0.3, 0.3, sin((time))) +
               circle(uvv, color2, color, drawCircle(0), 2.5, 0.25, sin((time + PI * 0.5)));

    return half4(c.r, c.g, c.b, c.a);
}

// Background shader functions
namespace Background {
    float blobShape(float2 uv, float2 center, float size, float blur) {
        float dist = length(uv - center);
        return smoothstep(size + blur, size - blur, dist);
    }

    float2 rotateUV(float2 uv, float angle, float2 center) {
        float2 c = center;
        float2x2 rotation = float2x2(
            cos(angle), -sin(angle),
            sin(angle), cos(angle)
        );
        return rotation * (uv - c) + c;
    }

    float4 createBgGradient(float2 uv, float2 center, float3 color, float opacity, float size) {
        float dist = length(uv - center);
        float alpha = smoothstep(size, size * 0.5, dist) * opacity;
        float inner = smoothstep(size * 0.2, size * 0.8, dist);
        float3 finalColor = mix(color * 1.2, color, inner);
        return float4(finalColor, alpha);
    }

    float4 createBgEdgeGradient(float2 uv, float angle, float3 color, float opacity, float time) {
        float2 dir = float2(cos(angle), sin(angle));
        float edgeDist = dot(uv - 0.5, dir) + 0.5;
        float gradient = smoothstep(0.0, 1.0, edgeDist);
        gradient = pow(gradient, 2.0);
        float wave = sin(edgeDist * 6.28 - time * 2.0) * 0.5 + 0.5;
        gradient = mix(gradient, gradient * wave, 0.3);
        return float4(color, gradient * opacity);
    }
}

[[ stitchable ]] half4 generateBackground(float2 position, half4 inputColor, float2 size, half4 colorA, half4 colorB, float time) {
    float2 uv = position / size;
    float3 brightColor = float3(colorA.r, colorA.g, colorA.b) * 1.2;
    float3 darkColor = float3(colorB.r, colorB.g, colorB.b) * 0.9;
    
    // Create four rotating edge gradients
    float angle1 = time * 0.3;
    float angle2 = angle1 + 2.57;
    float angle3 = angle1 + 3.14;
    float angle4 = angle1 + 4.71;
    
    float4 grad1 = Background::createBgEdgeGradient(uv, angle1, brightColor, 1, time);
    float4 grad2 = Background::createBgEdgeGradient(uv, angle2, mix(brightColor, darkColor, 0.3), 1, time);
    float4 grad3 = Background::createBgEdgeGradient(uv, angle3, mix(brightColor, darkColor, 0.7), 0.2, time);
    float4 grad4 = Background::createBgEdgeGradient(uv, angle4, darkColor, 0.9, time);
    
    float3 finalColor = darkColor * 0.4;
    
    finalColor = mix(finalColor, brightColor, grad1.a);
    finalColor = mix(finalColor, mix(brightColor, darkColor, 0.3), grad2.a);
    finalColor = mix(finalColor, mix(brightColor, darkColor, 0.7), grad3.a);
    finalColor = mix(finalColor, darkColor, grad4.a * 1.1);
    
    float totalGlow = (grad1.a + grad2.a) * 0.55;
    float3 glowColor = mix(brightColor, darkColor, 0.4);
    finalColor += glowColor * smoothstep(0.9, 1.0, totalGlow);
    
    return half4(finalColor.r, finalColor.g, finalColor.b, 0.5);
}
