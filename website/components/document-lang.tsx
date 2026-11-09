// The site has one root layout, which writes <html lang="en">. A page in another language wraps its
// content in an element carrying its lang (fonts and screen readers follow the nearest one) and
// mounts this to set <html lang> too while it is shown, restoring the previous value on the way out.
'use client';
import { useEffect } from 'react';

export function DocumentLang({ lang }: { lang: string }) {
  useEffect(() => {
    const root = document.documentElement;
    const previous = root.lang;
    root.lang = lang;
    return () => {
      root.lang = previous;
    };
  }, [lang]);
  return null;
}
