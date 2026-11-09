import type { Art } from '@/lib/rules';

export function Sticker({ art, className = '' }: { art: Art; className?: string }) {
  return <img src={art.src} alt={art.alt} width={art.width} height={art.height} className={`h-auto ${className}`} />;
}
