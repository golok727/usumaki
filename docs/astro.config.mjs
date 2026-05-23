// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightTypeDoc, { typeDocSidebarGroup } from 'starlight-typedoc';

export default defineConfig({
  site: 'https://uzumaki.run',
  integrations: [
    starlight({
      title: 'Uzumaki',
      logo: {
        light: './src/assets/logo.svg',
        dark: './src/assets/logo.svg',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/golok727/uzumaki',
        },
        {
          icon: 'x.com',
          label: 'X',
          href: 'https://x.com/golok727',
        },
      ],
      components: {
        Footer: './src/components/Footer.astro',
      },
      plugins: [
        starlightTypeDoc({
          entryPoints: ['../packages/uzumaki-types/uzumaki.d.ts'],
          output: 'api',
          tsconfig: './typedoc.tsconfig.json',
          sidebar: {
            label: 'API Reference',
            collapsed: true,
          },
          typeDoc: {
            entryPointStrategy: 'expand',
            excludePrivate: true,
            excludeProtected: true,
            hideGenerator: true,
          },
        }),
      ],
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        {
          label: 'Start Here',
          items: [
            { label: 'Installation', slug: 'guides/installation' },
            { label: 'Quick Start', slug: 'guides/quick-start' },
          ],
        },
        {
          label: 'Core Concepts',
          items: [
            { label: 'How Uzumaki Works', slug: 'concepts/how-it-works' },
            {
              label: 'React Without the Browser',
              slug: 'concepts/react-runtime',
            },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Style Native UI', slug: 'guides/styling' },
            { label: 'Theme with Variables', slug: 'guides/theming' },
            { label: 'Handle Events and State', slug: 'guides/events-state' },
            { label: 'Load Images and Resources', slug: 'guides/resources' },
            { label: 'Configure an App', slug: 'guides/configuration' },
            { label: 'Package for Distribution', slug: 'guides/building' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI', slug: 'reference/cli' },
            { label: 'Elements', slug: 'reference/elements' },
            { label: 'Props', slug: 'reference/props' },
            { label: 'Events', slug: 'reference/events' },
            { label: 'Window', slug: 'reference/window' },
            { label: 'Paths and Resources', slug: 'reference/paths' },
            { label: 'Runtime API', slug: 'reference/runtime-api' },
          ],
        },
        typeDocSidebarGroup,
      ],
      head: [
        {
          tag: 'script',
          content: `(() => {
  function pickPlatformTab() {
    var label = /Windows/i.test(navigator.userAgent || '') ? 'Windows' : 'macOS & Linux';
    document.querySelectorAll('starlight-tabs').forEach(function (group) {
      group.querySelectorAll('[role="tab"]').forEach(function (tab) {
        if ((tab.textContent || '').trim() === label && tab.getAttribute('aria-selected') !== 'true') {
          tab.click();
        }
      });
    });
  }
  if (document.readyState !== 'loading') pickPlatformTab();
  else document.addEventListener('DOMContentLoaded', pickPlatformTab);
})();`,
        },
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.googleapis.com',
          },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.gstatic.com',
            crossorigin: '',
          },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: 'https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700&family=Geist+Mono:wght@400;500;600;700&display=swap',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: 'https://uzumaki.run/social_preview.png',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:image:alt',
            content: 'Uzumaki',
          },
        },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:card',
            content: 'summary_large_image',
          },
        },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:image',
            content: 'https://uzumaki.run/social_preview.png',
          },
        },
      ],
    }),
  ],
});
