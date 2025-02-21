#include <metal_stdlib>
using namespace metal;

    // Constants
    constant float PI = 3.1415926535897932384626433832795;


    float blob(float2 uv, float2 center, float size, float blur) {
        float dist = length(uv - center);
        return smoothstep(size + blur, size - blur, dist);
    }

    float2 rotate(float2 uv, float angle, float2 center) {
        float2 c = center;
        float2x2 rotation = float2x2(
            cos(angle), -sin(angle),
            sin(angle), cos(angle)
        );
        return rotation * (uv - c) + c;
    }

    float4 createGradient(float2 uv, float2 center, float3 color, float opacity, float size) {
        float dist = length(uv - center);
        float alpha = smoothstep(size, size * 0.5, dist) * opacity;
        float inner = smoothstep(size * 0.2, size * 0.8, dist);
        float3 finalColor = mix(color * 1.2, color, inner);
        return float4(finalColor, alpha);
    }

    float4 createEdgeGradient(float2 uv, float angle, float3 color, float opacity, float time) {
        float2 dir = float2(cos(angle), sin(angle));
        float edgeDist = dot(uv - 0.5, dir) + 0.5;
        float gradient = smoothstep(0.0, 1.0, edgeDist);
        gradient = pow(gradient, 2.0);
        float wave = sin(edgeDist * 6.28 - time * 2.0) * 0.5 + 0.5;
        gradient = mix(gradient, gradient * wave, 0.3);
        return float4(color, gradient * opacity);
    }

    [[stitchable]] float4 generateBackground(float2 position, float2 size, float4 colorA, float4 colorB, float time) {
        float2 uv = position / size;
        float3 brightColor = float3(colorA.r, colorA.g, colorA.b);
        float3 darkColor = float3(colorB.r, colorB.g, colorB.b);
        
        // Create four rotating edge gradients
        float angle1 = time * 0.3;
        float angle2 = angle1 + 1.57;
        float angle3 = angle1 + 3.14;
        float angle4 = angle1 + 4.71;
        
        float4 grad1 = createEdgeGradient(uv, angle1, brightColor, 0.8, time);
        float4 grad2 = createEdgeGradient(uv, angle2, mix(brightColor, darkColor, 0.3), 0.7, time);
        float4 grad3 = createEdgeGradient(uv, angle3, mix(brightColor, darkColor, 0.7), 0.7, time);
        float4 grad4 = createEdgeGradient(uv, angle4, darkColor, 0.8, time);
        
        float3 finalColor = darkColor * 0.2;
        
        finalColor = mix(finalColor, brightColor, grad1.a);
        finalColor = mix(finalColor, mix(brightColor, darkColor, 0.3), grad2.a);
        finalColor = mix(finalColor, mix(brightColor, darkColor, 0.7), grad3.a);
        finalColor = mix(finalColor, darkColor, grad4.a);
        
        float totalGlow = (grad1.a + grad2.a) * 0.55;
        float3 glowColor = mix(brightColor, darkColor, 0.1);
        finalColor += glowColor * smoothstep(0.5, 1.0, totalGlow) * 0.9;
        
        return float4(finalColor.r, finalColor.g, finalColor.b, 1.0);
    }
