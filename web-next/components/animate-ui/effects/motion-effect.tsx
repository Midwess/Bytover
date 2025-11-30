'use client';

import * as React from 'react';
import {
  AnimatePresence,
  motion,
  useInView,
  type HTMLMotionProps,
  type UseInViewOptions,
  type Transition,
  type Variant,
} from 'motion/react';

type MotionEffectProps = HTMLMotionProps<'div'> & {
  children: React.ReactNode;
  className?: string;
  transition?: Transition;
  delay?: number;
  inView?: boolean;
  inViewMargin?: UseInViewOptions['margin'];
  inViewOnce?: boolean;
  blur?: string | boolean;
  slide?:
  | {
    direction?: 'up' | 'down' | 'left' | 'right';
    offset?: number;
  }
  | boolean;
  fade?: { initialOpacity?: number; opacity?: number } | boolean;
  zoom?:
  | {
    initialScale?: number;
    scale?: number;
  }
  | boolean;
};

function MotionEffect({
  ref,
  children,
  className,
  transition = { type: 'tween', stiffness: 200, damping: 20 },
  delay = 0,
  inView = false,
  inViewMargin = '0px',
  inViewOnce = true,
  blur = false,
  slide = false,
  fade = false,
  zoom = false,
  ...props
}: MotionEffectProps) {
  const localRef = React.useRef<HTMLDivElement>(null);
  React.useImperativeHandle(ref, () => localRef.current as HTMLDivElement);

  const inViewResult = useInView(localRef, {
    once: inViewOnce,
    margin: inViewMargin,
  });
  const isInView = !inView || inViewResult;

  const { hiddenVariant, visibleVariant } = React.useMemo(() => {
    const hidden: Variant = {};
    const visible: Variant = {};

    if (slide) {
      const offset = typeof slide === 'boolean' ? 100 : (slide.offset ?? 100);
      const direction =
        typeof slide === 'boolean' ? 'left' : (slide.direction ?? 'left');
      const axis = direction === 'up' || direction === 'down' ? 'y' : 'x';
      hidden[axis] =
        direction === 'left' || direction === 'up' ? -offset : offset;
      visible[axis] = 0;
    }

    if (fade) {
      hidden.opacity =
        typeof fade === 'boolean' ? 0 : (fade.initialOpacity ?? 0);
      visible.opacity = typeof fade === 'boolean' ? 1 : (fade.opacity ?? 1);
    }

    if (zoom) {
      hidden.scale =
        typeof zoom === 'boolean' ? 0.5 : (zoom.initialScale ?? 0.5);
      visible.scale = typeof zoom === 'boolean' ? 1 : (zoom.scale ?? 1);
    }

    if (blur) {
      hidden.filter =
        typeof blur === 'boolean' ? 'blur(10px)' : `blur(${blur})`;
      visible.filter = 'blur(0px)';
    }

    return { hiddenVariant: hidden, visibleVariant: visible };
  }, [slide, fade, zoom, blur]);

  return (
    <AnimatePresence>
      <motion.div
        ref={localRef}
        data-slot="motion-effect"
        initial="hidden"
        animate={isInView ? 'visible' : 'hidden'}
        exit="hidden"
        variants={{
          hidden: hiddenVariant,
          visible: visibleVariant,
        }}
        transition={{
          ...transition,
          delay: (transition?.delay ?? 0) + delay,
        }}
        className={className}
        {...props}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}

export { MotionEffect, type MotionEffectProps };
