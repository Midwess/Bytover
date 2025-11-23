import * as React from 'react';

import { cn } from '@/lib/utils';

type MouseGravity = 'attract' | 'repel';
type GlowAnimation = 'instant' | 'ease' | 'spring';
type StarsInteractionType = 'bounce' | 'merge';

type GravityStarsProps = {
  starsCount?: number;
  starsSize?: number;
  starsOpacity?: number;
  glowIntensity?: number;
  glowAnimation?: GlowAnimation;
  movementSpeed?: number;
  mouseInfluence?: number;
  mouseGravity?: MouseGravity;
  gravityStrength?: number;
  starsInteraction?: boolean;
  starsInteractionType?: StarsInteractionType;
  waveSpeed?: number;
  waveWidth?: number;
  waveAmplitude?: number;
} & React.ComponentProps<'div'>;

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  opacity: number;
  baseOpacity: number;
  mass: number;
  glowMultiplier?: number;
  glowVelocity?: number;
  length?: number;
  waveOpacity?: number;
  waveIntensity?: number;
};

function GravityStarsBackground({
  starsCount = 205,
  starsSize = 2,
  starsOpacity = 0.75,
  glowIntensity = 35,
  glowAnimation = 'ease',
  movementSpeed = 0.4,
  mouseInfluence = 100,
  mouseGravity = 'attract',
  gravityStrength = 75,
  starsInteraction = false,
  starsInteractionType = 'bounce',
  waveSpeed = 2,
  waveWidth = 40,
  waveAmplitude = 1.5,
  className,
  ...props
}: GravityStarsProps) {
  const containerRef = React.useRef<HTMLDivElement | null>(null);
  const canvasRef = React.useRef<HTMLCanvasElement | null>(null);
  const animRef = React.useRef<number | null>(null);
  const starsRef = React.useRef<Particle[]>([]);
  const mouseRef = React.useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const globalMouseRef = React.useRef<{ x: number; y: number }>({ 
    x: typeof window !== 'undefined' ? window.innerWidth / 2 : 0, 
    y: typeof window !== 'undefined' ? window.innerHeight / 2 : 0 
  });
  const mouseVelocityRef = React.useRef<{ x: number; y: number; magnitude: number }>({ x: 0, y: 0, magnitude: 0 });
  const waveTimeRef = React.useRef(0);
  const lastMouseRef = React.useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const waveHeightMapRef = React.useRef<Map<string, number>>(new Map());
  const wavePropagationRef = React.useRef<Array<{ x: number; y: number; time: number; strength: number }>>([]);
  const [dpr, setDpr] = React.useState(1);
  const [canvasSize, setCanvasSize] = React.useState({
    width: 800,
    height: 600,
  });

  const readColor = React.useCallback(() => {
    const el = containerRef.current;
    if (!el) return '#ffffff';
    const cs = getComputedStyle(el);
    return cs.color || '#ffffff';
  }, []);

  const initStars = React.useCallback(
    (w: number, h: number) => {
      starsRef.current = Array.from({ length: starsCount }).map(() => {
        const angle = Math.random() * Math.PI * 2;
        const speed = movementSpeed * (0.5 + Math.random() * 0.5);
        return {
          x: Math.random() * w,
          y: Math.random() * h,
          vx: Math.cos(angle) * speed,
          vy: Math.sin(angle) * speed,
          size: Math.random() * starsSize + 1,
          opacity: starsOpacity,
          baseOpacity: starsOpacity,
          mass: Math.random() * 0.5 + 0.5,
          glowMultiplier: 1,
          glowVelocity: 0,
          length: Math.random() * 8 + 4,
          waveOpacity: 0,
          waveIntensity: 0,
        };
      });
    },
    [starsCount, movementSpeed, starsOpacity, starsSize],
  );

  const redistributeStars = React.useCallback((w: number, h: number) => {
    starsRef.current.forEach((p) => {
      p.x = Math.random() * w;
      p.y = Math.random() * h;
    });
  }, []);

  const resizeCanvas = React.useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const rect = container.getBoundingClientRect();
    const nextDpr = Math.max(1, Math.min(window.devicePixelRatio || 1, 2));
    setDpr(nextDpr);
    canvas.width = Math.max(1, Math.floor(rect.width * nextDpr));
    canvas.height = Math.max(1, Math.floor(rect.height * nextDpr));
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    setCanvasSize({ width: rect.width, height: rect.height });
    if (starsRef.current.length === 0) {
      initStars(rect.width, rect.height);
    } else {
      redistributeStars(rect.width, rect.height);
    }
  }, [initStars, redistributeStars]);

  const handlePointerMove = React.useCallback(
    (e: React.MouseEvent | React.TouchEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      let clientX = 0;
      let clientY = 0;
      if ('touches' in e) {
        const t = e.touches[0];
        if (!t) return;
        clientX = t.clientX;
        clientY = t.clientY;
      } else {
        clientX = e.clientX;
        clientY = e.clientY;
      }
      const newX = clientX - rect.left;
      const newY = clientY - rect.top;
      
      // Calculate mouse velocity for wave intensity
      const dx = newX - mouseRef.current.x;
      const dy = newY - mouseRef.current.y;
      const magnitude = Math.hypot(dx, dy);
      
      // Add wave drop at cursor position (perturbance model from jquery.ripples)
      const dropRadius = 25;
      const strength = Math.min(1, magnitude * 0.05); // Clamp strength 0-1
      
      if (magnitude > 0.5) {
        wavePropagationRef.current.push({
          x: newX,
          y: newY,
          time: 0,
          strength: strength,
        });
      }
      
      mouseRef.current = { x: newX, y: newY };
      globalMouseRef.current = { x: clientX, y: clientY };
      mouseVelocityRef.current = { x: dx, y: dy, magnitude };
    },
    [],
  );

  // Global mouse tracking
  React.useEffect(() => {
    const handleGlobalMouseMove = (e: MouseEvent) => {
      globalMouseRef.current = { x: e.clientX, y: e.clientY };
    };

    document.addEventListener('mousemove', handleGlobalMouseMove);
    return () => {
      document.removeEventListener('mousemove', handleGlobalMouseMove);
    };
  }, []);

  const updateStars = React.useCallback(() => {
    const w = canvasSize.width;
    const h = canvasSize.height;
    const mouse = mouseRef.current;
    const velocity = mouseVelocityRef.current;

    // Update wave propagation (decay and spread)
    for (let i = wavePropagationRef.current.length - 1; i >= 0; i--) {
      const wave = wavePropagationRef.current[i];
      wave.time += 0.05;
      // Remove waves after they've propagated far enough
      if (wave.time > 3) {
        wavePropagationRef.current.splice(i, 1);
      }
    }

    for (let i = 0; i < starsRef.current.length; i++) {
      const p = starsRef.current[i];

      const dx = mouse.x - p.x;
      const dy = mouse.y - p.y;
      const dist = Math.hypot(dx, dy);

      // Wave effect based on mouse movement
      if (dist < mouseInfluence && dist > 0) {
        const force = (mouseInfluence - dist) / mouseInfluence;
        const nx = dx / dist;
        const ny = dy / dist;
        const g = force * (gravityStrength * 0.001);

        if (mouseGravity === 'attract') {
          p.vx += nx * g;
          p.vy += ny * g;
        } else if (mouseGravity === 'repel') {
          p.vx -= nx * g;
          p.vy -= ny * g;
        }

        p.opacity = Math.min(1, p.baseOpacity + force * 0.4);

        const targetGlow = 1 + force * 2;
        const currentGlow = p.glowMultiplier || 1;

        if (glowAnimation === 'instant') {
          p.glowMultiplier = targetGlow;
        } else if (glowAnimation === 'ease') {
          const ease = 0.15;
          p.glowMultiplier = currentGlow + (targetGlow - currentGlow) * ease;
        } else {
          const spring = (targetGlow - currentGlow) * 0.2;
          const damping = 0.85;
          p.glowVelocity = (p.glowVelocity || 0) * damping + spring;
          p.glowMultiplier = currentGlow + (p.glowVelocity || 0);
        }

        // Wave intensity based on mouse velocity
        const waveIntensity = Math.min(1, velocity.magnitude * 0.15);
        p.waveIntensity = Math.max(p.waveIntensity || 0, waveIntensity);
        
        // Apply perturbance from active wave propagations
        for (const wave of wavePropagationRef.current) {
          const waveDx = p.x - wave.x;
          const waveDy = p.y - wave.y;
          const waveDist = Math.hypot(waveDx, waveDy);
          const waveRadius = wave.time * 30; // Propagation speed
          const perturbRadius = 40;
          
          if (waveDist < waveRadius + perturbRadius && waveDist > waveRadius - perturbRadius) {
            // Gaussian envelope for smooth wave
            const diff = Math.abs(waveDist - waveRadius);
            const envelope = Math.exp(-diff * diff / (2 * perturbRadius * perturbRadius));
            const perturbAmount = wave.strength * envelope * 0.08;
            
            const nx = waveDx / (waveDist + 0.001);
            const ny = waveDy / (waveDist + 0.001);
            p.vx += nx * perturbAmount;
            p.vy += ny * perturbAmount;
          }
        }
      } else {
        p.opacity = Math.max(p.baseOpacity * 0.3, p.opacity - 0.02);
        p.waveIntensity = (p.waveIntensity || 0) * 0.95;
        
        const targetGlow = 1;
        const currentGlow = p.glowMultiplier || 1;
        if (glowAnimation === 'instant') {
          p.glowMultiplier = targetGlow;
        } else if (glowAnimation === 'ease') {
          const ease = 0.08;
          p.glowMultiplier = Math.max(
            1,
            currentGlow + (targetGlow - currentGlow) * ease,
          );
        } else {
          const spring = (targetGlow - currentGlow) * 0.15;
          const damping = 0.9;
          p.glowVelocity = (p.glowVelocity || 0) * damping + spring;
          p.glowMultiplier = Math.max(1, currentGlow + (p.glowVelocity || 0));
        }
      }

      if (starsInteraction) {
        for (let j = i + 1; j < starsRef.current.length; j++) {
          const o = starsRef.current[j];
          const dx2 = o.x - p.x;
          const dy2 = o.y - p.y;
          const d = Math.hypot(dx2, dy2);
          const minD = p.size + o.size + 5;
          if (d < minD && d > 0) {
            if (starsInteractionType === 'bounce') {
              const nx = dx2 / d;
              const ny = dy2 / d;
              const rvx = p.vx - o.vx;
              const rvy = p.vy - o.vy;
              const speed = rvx * nx + rvy * ny;
              if (speed < 0) continue;
              const impulse = (2 * speed) / (p.mass + o.mass);
              p.vx -= impulse * o.mass * nx;
              p.vy -= impulse * o.mass * ny;
              o.vx += impulse * p.mass * nx;
              o.vy += impulse * p.mass * ny;
              const overlap = minD - d;
              const sx = nx * overlap * 0.5;
              const sy = ny * overlap * 0.5;
              p.x -= sx;
              p.y -= sy;
              o.x += sx;
              o.y += sy;
            } else {
              const mergeForce = (minD - d) / minD;
              p.glowMultiplier = (p.glowMultiplier || 1) + mergeForce * 0.5;
              o.glowMultiplier = (o.glowMultiplier || 1) + mergeForce * 0.5;
              const af = mergeForce * 0.01;
              p.vx += dx2 * af;
              p.vy += dy2 * af;
              o.vx -= dx2 * af;
              o.vy -= dy2 * af;
            }
          }
        }
      }

      p.x += p.vx;
      p.y += p.vy;

      p.vx += (Math.random() - 0.5) * 0.001;
      p.vy += (Math.random() - 0.5) * 0.001;

      p.vx *= 0.999;
      p.vy *= 0.999;

      if (p.x < 0) p.x = w;
      if (p.x > w) p.x = 0;
      if (p.y < 0) p.y = h;
      if (p.y > h) p.y = 0;
    }
  }, [
    canvasSize.width,
    canvasSize.height,
    mouseInfluence,
    mouseGravity,
    gravityStrength,
    glowAnimation,
    starsInteraction,
    starsInteractionType,
  ]);

  const drawStars = React.useCallback(
    (ctx: CanvasRenderingContext2D) => {
      ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
      const color = readColor();
      const globalMouse = globalMouseRef.current;
      const container = containerRef.current;
      if (!container) return;
      
      const rect = container.getBoundingClientRect();
      const mouseX = globalMouse.x - rect.left;
      const mouseY = globalMouse.y - rect.top;
      const clipRadius = Math.min(window.innerWidth, window.innerHeight) * 0.3;
      
      // Update wave time for animation
      waveTimeRef.current += waveSpeed * 0.02;
      
      for (const p of starsRef.current) {
        const dist = Math.hypot(mouseX - p.x, mouseY - p.y);
        
        // Only process if within clipRadius of mouse
        if (dist > clipRadius) continue;
        
        // Calculate angle from mouse to particle
        const angle = Math.atan2(p.y - mouseY, p.x - mouseX);
        
        // Enhanced wave effect with proper perturbance propagation
        const normalizedDist = dist / clipRadius;
        const wavePhase = (normalizedDist * waveWidth - waveTimeRef.current * waveSpeed + angle * 1.5) % (Math.PI * 2);
        const waveValue = (Math.sin(wavePhase) + 1) / 2;
        
        // Calculate wave perturbance from propagating ripples
        let perturbanceFromRipples = 0;
        for (const wave of wavePropagationRef.current) {
          const rippleDx = p.x - wave.x;
          const rippleDy = p.y - wave.y;
          const rippleDist = Math.hypot(rippleDx, rippleDy);
          const waveRadius = wave.time * 30;
          const perturbRadius = 50;
          
          if (rippleDist < waveRadius + perturbRadius && rippleDist > waveRadius - perturbRadius) {
            const diff = Math.abs(rippleDist - waveRadius);
            const envelope = Math.exp(-diff * diff / (2 * perturbRadius * perturbRadius));
            perturbanceFromRipples += wave.strength * envelope * 0.6;
          }
        }
        
        // Wave band that follows mouse movement
        const waveThreshold = 0.2;
        const waveIntensity = p.waveIntensity || 0;
        const combinedWave = (1 - normalizedDist) * Math.max(waveValue, waveIntensity + perturbanceFromRipples);
        
        if (combinedWave < waveThreshold) continue;
        
        // Calculate opacity and intensity based on wave position and velocity
        const waveOpacity = Math.min(1, (combinedWave - waveThreshold) / (1 - waveThreshold));
        const amplitudeModulation = waveAmplitude * (1 + (waveIntensity + perturbanceFromRipples) * 0.5);
        
        ctx.save();
        ctx.shadowColor = color;
        ctx.shadowBlur = glowIntensity * (p.glowMultiplier || 1) * 2 * waveOpacity * amplitudeModulation;
        ctx.globalAlpha = p.opacity * waveOpacity * amplitudeModulation;
        ctx.fillStyle = color;
        
        // Draw capsule shape
        const length = (p.length || 6) * dpr;
        const particleRadius = p.size * dpr;
        const particleAngle = Math.atan2(p.vy, p.vx);
        
        ctx.beginPath();
        ctx.translate(p.x * dpr, p.y * dpr);
        ctx.rotate(particleAngle);
        
        // Draw capsule (rounded rectangle with semicircles on ends)
        const halfLength = length / 2;
        ctx.arc(-halfLength, 0, particleRadius, Math.PI / 2, -Math.PI / 2, false);
        ctx.arc(halfLength, 0, particleRadius, -Math.PI / 2, Math.PI / 2, false);
        ctx.closePath();
        ctx.fill();
        
        ctx.restore();
      }
    },
    [dpr, glowIntensity, waveSpeed, waveWidth, waveAmplitude, readColor],
  );

  const animate = React.useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    updateStars();
    drawStars(ctx);
    animRef.current = requestAnimationFrame(animate);
  }, [updateStars, drawStars]);

  React.useEffect(() => {
    resizeCanvas();
    const container = containerRef.current;
    const ro =
      typeof ResizeObserver !== 'undefined'
        ? new ResizeObserver(resizeCanvas)
        : null;
    if (container && ro) ro.observe(container);
    const onResize = () => resizeCanvas();
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('resize', onResize);
      if (ro && container) ro.disconnect();
    };
  }, [resizeCanvas]);

  React.useEffect(() => {
    if (starsRef.current.length === 0) {
      initStars(canvasSize.width, canvasSize.height);
    } else {
      starsRef.current.forEach((p) => {
        p.baseOpacity = starsOpacity;
        p.opacity = starsOpacity;
        const spd = Math.hypot(p.vx, p.vy);
        if (spd > 0) {
          const ratio = movementSpeed / spd;
          p.vx *= ratio;
          p.vy *= ratio;
        }
      });
    }
  }, [
    starsCount,
    starsOpacity,
    movementSpeed,
    canvasSize.width,
    canvasSize.height,
    initStars,
  ]);

  React.useEffect(() => {
    if (animRef.current) cancelAnimationFrame(animRef.current);
    animRef.current = requestAnimationFrame(animate);
    return () => {
      if (animRef.current) cancelAnimationFrame(animRef.current);
      animRef.current = null;
    };
  }, [animate]);

  const [mousePosition, setMousePosition] = React.useState({ 
    x: typeof window !== 'undefined' ? window.innerWidth / 2 : 0, 
    y: typeof window !== 'undefined' ? window.innerHeight / 2 : 0 
  });

  // Update clip-path position
  React.useEffect(() => {
    const updateClipPath = () => {
      setMousePosition({ 
        x: globalMouseRef.current.x, 
        y: globalMouseRef.current.y 
      });
    };

    const interval = setInterval(updateClipPath, 16);
    return () => clearInterval(interval);
  }, []);

  return (
    <div
      ref={containerRef}
      data-slot="gravity-stars-background"
      className={cn('relative size-full overflow-hidden', className)}
      onMouseMove={(e) => handlePointerMove(e)}
      onTouchMove={(e) => handlePointerMove(e)}
      style={{
        clipPath: `circle(30vw at ${mousePosition.x}px ${mousePosition.y}px)`,
        WebkitClipPath: `circle(30vw at ${mousePosition.x}px ${mousePosition.y}px)`,
      }}
      {...props}
    >
      <canvas ref={canvasRef} className="block w-full h-full" />
    </div>
  );
}

export { GravityStarsBackground, type GravityStarsProps };