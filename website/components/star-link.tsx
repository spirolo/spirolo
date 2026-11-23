// The one ask the site makes besides the install: a star on the repository. A plain link to the
// repository, without a live count, so the page stays static and calls nothing.
import { product } from '@/lib/product';

export function StarIcon({ className = 'size-4' }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" className={className} fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden>
      <path d="M8 1.9l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.7l-3.8 2 .7-4.3-3.1-3 4.3-.6z" strokeLinejoin="round" />
    </svg>
  );
}

export function StarLink({ children }: { children: React.ReactNode }) {
  return (
    <a
      href={product.repoUrl}
      className="inline-flex items-center gap-2 rounded-xl border border-glow/50 px-4 py-2 text-sm font-medium text-glow transition-colors hover:bg-glow hover:text-night"
    >
      <StarIcon />
      {children}
    </a>
  );
}
