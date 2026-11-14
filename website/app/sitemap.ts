import type { MetadataRoute } from 'next';
import { siteUrl } from '@/lib/site';
import { source } from '@/lib/source';

export const dynamic = 'force-static';

// The two home pages name each other as language versions, as their own metadata does.
const homeLanguages = { en: siteUrl, 'zh-CN': `${siteUrl}/zh` };

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url: siteUrl, alternates: { languages: homeLanguages } },
    { url: `${siteUrl}/zh`, alternates: { languages: homeLanguages } },
    ...source.getPages().map((page) => ({ url: `${siteUrl}${page.url}` })),
  ];
}
