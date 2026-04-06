import { defineConfig } from 'vitepress'

// Set base to /upslim/ when building for GitHub Pages
const base = process.env.GITHUB_ACTIONS ? '/upslim/' : '/'

export default defineConfig({
  base,
  lang: 'en-US',
  title: 'UpSlim',
  description: 'Minimal, efficient uptime monitoring server written in Rust.',

  appearance: 'dark',
  lastUpdated: true,

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}logo.svg` }],
  ],

  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'UpSlim',

    nav: [
      { text: 'Guide', link: '/guide/', activeMatch: '/guide/' },
      { text: 'Alerting', link: '/alerting/', activeMatch: '/alerting/' },
      { text: 'Reference', link: '/reference/conditions', activeMatch: '/reference/' },
      {
        text: 'Changelog',
        link: 'https://github.com/AbianS/upslim/releases',
      },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is UpSlim?', link: '/guide/' },
            { text: 'Installation', link: '/guide/installation' },
          ],
        },
        {
          text: 'Configuration',
          items: [
            { text: 'Config File', link: '/guide/configuration' },
            { text: 'Monitors', link: '/guide/monitors' },
          ],
        },
      ],
      '/alerting/': [
        {
          text: 'Alerting',
          items: [
            { text: 'Overview', link: '/alerting/' },
            { text: 'Slack', link: '/alerting/slack' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'Conditions DSL', link: '/reference/conditions' },
            { text: 'Docker & Deploy', link: '/reference/docker' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/AbianS/upslim' },
    ],

    editLink: {
      pattern: 'https://github.com/AbianS/upslim/edit/main/packages/docs/:path',
      text: 'Edit this page on GitHub',
    },

    lastUpdated: {
      text: 'Last updated',
      formatOptions: {
        dateStyle: 'medium',
      },
    },

    search: {
      provider: 'local',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present UpSlim contributors',
    },

    outline: {
      level: [2, 3],
      label: 'On this page',
    },

    docFooter: {
      prev: 'Previous',
      next: 'Next',
    },
  },
})
