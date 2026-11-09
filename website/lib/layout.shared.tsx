import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { product } from './product';

export function Wordmark() {
  return (
    <img src="/brand/logo/lockup-horizontal-glow-motion.svg" alt={product.name} width={94} height={32} />
  );
}

export function baseOptions(): BaseLayoutProps {
  return {
    nav: { title: <Wordmark /> },
    links: [{ text: 'Docs', url: '/docs', active: 'nested-url' }],
    githubUrl: product.repoUrl,
    themeSwitch: { enabled: false },
  };
}
