---
title: FAQ
---

# Frequently asked questions

- [Spell-check doesn't work; how do I enable it?](#spell-check-doesnt-work-how-do-i-enable-it)
- [Some of my Markdown elements aren't highlighted](#some-of-my-markdown-elements-arent-highlighted)
- [Which elements of Markdown are supported?](#which-elements-of-markdown-are-supported)
- [Autocompletion doesn't work](#autocompletion-doesnt-work)
- [Syntax-highlighting is broken after uninstall](#syntax-highlighting-is-broken-after-uninstall)
- [Trailing whitespace is automatically removed, but I don't want that](#trailing-whitespace-is-automatically-removed-but-i-dont-want-that)

## Spell-check doesn't work; how do I enable it?

The core-package `spell-check` doesn't scan documents in the `text.md` by default. You can easily add this yourself:

- Open the Atom settings, and find the Packages tab
- Search for the `spell-check` package; you can find it under the Core Packages
- Open the settings for `spell-check`
- Append `text.md` to the list of grammars (make sure the scopes are comma-separated)
- Reload Atom to make sure the updated settings take effect

## Some of my Markdown elements aren't highlighted

`language-markdown` parses your Markdown document; it does not directly color the different elements. This is done by the syntax-theme you are using. There's a good chance that your syntax-theme doesn't support all the different elements that `language-markdown` recognizes. You can ask the author of the theme to add better support for `language-markdown`, or [add styles to your custom stylesheet](http://flight-manual.atom.io/using-atom/sections/basic-customization/#style-tweaks). You can also try one of the tried and tested syntax-themes featured above. If you can't get it to work, feel free to [open an issue](https://github.com/burodepeper/language-markdown/issues/new/), and I'll see what I can do.

## Which elements of Markdown are supported?

Because there is no clear Markdown standard, I've chosen to follow the [CommonMark Spec](http://spec.commonmark.org/) as closely as possible within the Atom environment. On top of that, I've implemented support for a few extensions: Github Flavored Markdown, Markdown Extra, CriticMark, Front Matter, and R Markdown. Together, I believe these specs cover a solid 98% of your day-to-day Markdown needs. If you feel that an element is missing, please [open an issue](https://github.com/burodepeper/language-markdown/issues/new/).

#### Notes on implementation

- Raw `html` is included when you have the default `language-html` grammar enabled
- The Github Flavored `task-lists` are implemented as part of 'normal' `lists`
- Setext-headers (underlined-headers) are not supported
- `indented-code-blocks` have been disabled to prevent false-positives; use `fenced-code-blocks` instead ([more details](https://github.com/burodepeper/language-markdown/issues/88#issuecomment-183344420))
- Github tables require pipes at the start of each line, and cells need a padding of at least one space; this is a suggested convention to prevent false positives

## Autocompletion doesn't work

Autocompletion doesn't work out-of-the-box with Markdown documents. It is possible to enable it, but it might need some tinkering. In the `autocomplete-plus` settings, make sure that Markdown files aren't blacklisted. Additionally, it might help to switch the default provider to Fuzzy.

For Atom to index your Markdown documents as symbols, you have to add the following to your `config.cson`:

```coffee
'.text.md':
    autocomplete:
        symbols:
            constant:
                selector: "*"
                typePriority: 1
```

You can find additional information in [this issue](https://github.com/burodepeper/language-markdown/issues/150).

## Syntax-highlighting is broken after uninstall

The core-package `language-gfm` is automatically disabled (unless you've enabled the setting that prevents this) when using `language-markdown` to avoid any conflicts. Because `language-markdown` is intended as a drop-in replacement you most likely won't need both anyway. However, if you uninstall `language-markdown`, `language-gfm` doesn't automatically get re-activated. There's no API available to do this, so you'll have to re-activate `language-gfm` manually, which is quite easy.

1. Open the "Settings" and go to the "Packages" tab
2. Search for `language-gfm`
3. Click `Enable` to re-activate it
4. You probably want to reload Atom to make sure the change takes effect

## Trailing whitespace is automatically removed, but I don't want that

By default, Atom removes all trailing whitespace when a file is saved. You can disable it by setting the following flag in your `config.cson` for the `.md.text` scope. For more background, see [#115](https://github.com/burodepeper/language-markdown/issues/115).

```coffee
'*':
  # all current config
'.md.text':
  whitespace:
    removeTrailingWhitespace: false
```