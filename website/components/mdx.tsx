import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { HiddenReason, RuleStatusCard, RuleTable } from '@/components/rule-status';
import { Terminal } from '@/components/terminal';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    RuleStatusCard,
    HiddenReason,
    RuleTable,
    Terminal,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
