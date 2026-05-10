'use client';

/**
 * styled-jsx SSR registry for the Next.js App Router.
 *
 * Without this, every `<style jsx>` block on the page is injected client-side
 * during hydration — meaning the initial HTML renders with no component
 * styles, which produced a ~0.92 CLS shift on the homepage when JS finally
 * mounted and the layout collapsed into shape.
 *
 * The registry hooks into `useServerInsertedHTML` to flush styled-jsx's
 * collected `<style>` tags into the SSR response stream, so the browser
 * has the rules available on first paint.
 *
 * Mirrors the pattern documented in the Next.js styling guide:
 * https://nextjs.org/docs/app/building-your-application/styling/css-in-js#styled-jsx
 */

import { useServerInsertedHTML } from 'next/navigation';
import { type ReactNode, useState } from 'react';
import { createStyleRegistry, StyleRegistry } from 'styled-jsx';

export function StyledJsxRegistry({ children }: { children: ReactNode }) {
  // Lazy initial state so the registry is created exactly once per render.
  const [registry] = useState(() => createStyleRegistry());

  useServerInsertedHTML(() => {
    const styles = registry.styles();
    registry.flush();
    return <>{styles}</>;
  });

  return <StyleRegistry registry={registry}>{children}</StyleRegistry>;
}
