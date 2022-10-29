const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Valence.rs',
  tagline: 'Collection of rust crates to create a minecraft server',
  url: 'https://valence.rs',
  baseUrl: '/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.ico',
  organizationName: 'valence-rs',
  projectName: 'valence',
  trailingSlash: false,
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: '',
        logo: {
          alt: 'Valence Logo',
          src: 'img/logo.svg',
        },
        items: [
          {
            type: 'doc',
            docId: 'Getting Started',
            position: 'left',
            label: 'Guide',
          },
          {to: '/faq', label: 'FAQ', position: 'left'},
          {to: '/releases', label: 'Release Notes', position: 'right'},
          {
            href: 'https://github.com/valence-rs/valence',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              {
                label: 'Guide',
                to: '/docs/Getting Started',
              },
            ],
          },
          {
            title: 'Community',
            items: [
              {
                label: 'Patreon',
                href: 'https://patreon.com/rj00a',
              },
              {
                label: 'Discord',
                href: 'https://discord.gg/8Fqqy9XrYb',
              },
              {
                label: 'GitHub Sponsors',
                href: 'https://github.com/sponsors/rj00a',
              },
            ],
          },
          {
            title: 'More',
            items: [
              {
                label: 'Crates.io',
                href: 'https://crates.io/crates/valence',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Ryan Johnson et. al.`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },
    }),
};

module.exports = config,{
  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: false,
    },
  },
};