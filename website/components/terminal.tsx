// Renders a real `check` report. The text is never edited: lines are only coloured by what they
// are, and the one line named by `finding` gets the amber that marks a finding.

function lineClass(line: string): string {
  if (line.startsWith('── ')) return 'text-glow font-medium';
  if (line.startsWith('● ')) return 'text-fd-foreground font-medium';
  if (line.startsWith('Health ')) return 'text-fd-foreground font-medium';
  if (line.startsWith('✓ ') || line.startsWith(' hidden') || line.startsWith(' note')) return 'text-fd-muted-foreground';
  if (line.includes('· spirolo explain ') || line.trimStart().startsWith('To gate it:')) return 'text-fd-muted-foreground';
  return 'text-fd-foreground/85';
}

export function Terminal({ output, command, finding }: { output: string; command: string; finding?: string }) {
  const lines = output.split('\n');

  return (
    <div className="overflow-hidden rounded-2xl border border-fd-border bg-[#0f0b15]">
      <div className="flex items-center gap-2 border-b border-fd-border px-4 py-2.5">
        <span className="size-2.5 rounded-full bg-plum" />
        <span className="size-2.5 rounded-full bg-plum/60" />
        <span className="size-2.5 rounded-full bg-plum/30" />
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-[12.5px] leading-relaxed sm:p-5">
        <code>
          <span className="text-fd-muted-foreground">$ </span>
          <span className="text-fd-foreground">{command}</span>
          {'\n'}
          {lines.map((line, index) => {
            const isFinding = finding !== undefined && line.includes(finding);
            return (
              <span key={index} className={isFinding ? 'text-amber' : lineClass(line)}>
                {line}
                {'\n'}
              </span>
            );
          })}
        </code>
      </pre>
    </div>
  );
}
