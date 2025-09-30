#include <metal_stdlib>
using namespace metal;

constant half PI = 3.141592653589793h;

half4 circle(half2 uv, half3 mainColor, half3 subColor, half2 center, half sharpness, half radius, half time) {
    // Cache delta position calculation
    half2 deltaPos = uv - center;
    half dist = length(deltaPos);
    
    // Calculate angle for rotating glow effect
    half angle = atan2(deltaPos.y, deltaPos.x);
    half rotatingGlow = 0.3h + 0.3h * sin(angle + time * 2.0h);
    
    // Optimized border calculation with sharper transitions
    half borderWidth = 0.01h; // Reduced for sharper edges
    half borderMask = step(radius - borderWidth, dist) * step(dist, radius + borderWidth);
    half borderFalloff = 1.0h - abs(dist - radius) / borderWidth;
    half border = borderMask * borderFalloff * sharpness;
    
    // More efficient glow calculation with increased falloff
    half distFromBorder = abs(dist - radius);
    half glowIntensity = exp(-21.0h * distFromBorder) * (0.3h + 0.7h * rotatingGlow) * 1.8h;
    
    // Sharper inner fill with tighter range
    half innerFill = 1.0h - smoothstep(radius * 0.98h, radius, dist);
    
    // Combine effects efficiently
    half intensity = max(max(glowIntensity, border), innerFill * 0.25h);
    
    // Optimized color mixing
    half3 glowColor = mix(mainColor * 0.8h, subColor, rotatingGlow);
    return half4(glowColor * intensity, intensity);
}

// Inline function to avoid call overhead
inline half2 getCenter() {
    return half2(0.5h, 0.5h);
}

[[ stitchable ]] half4 circleWave(float2 position, half4 inputColor, float2 size, half4 subColor, float time) {
    // Convert to half precision early and cache calculations
    half2 uv = half2(position / size);
    half3 color1 = inputColor.rgb;
    half3 color2 = subColor.rgb;
    half2 center = getCenter();
    
    // Cache time calculations to avoid redundant sin computations
    half t1 = half(sin(time));
    half t2 = half(sin(time + PI * 0.5h));
    
    // Create two circles with optimized parameters
    half4 c1 = circle(uv, color1, color2, center, 0.7h, 0.18h, t1);
    half4 c2 = circle(uv, color2, color1, center, 2.5h, 0.13h, t2);
    
    // Efficient blending
    return c1 + c2 * 1.8h;
}

