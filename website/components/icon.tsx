/**
 * An interface icon from `../branding/elements/icons`: one stroke, drawn from a prop in the scenes.
 * It is painted through a mask so it takes the text colour around it, like the bead bullets.
 */
export function Icon({ src, className = '' }: { src: string; className?: string }) {
  return (
    <span
      aria-hidden
      className={`inline-block shrink-0 bg-current ${className}`}
      style={{ mask: `url('${src}') center / contain no-repeat` }}
    />
  );
}
