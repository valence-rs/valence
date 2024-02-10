# Valence.rs

This website is built using the Static Site Generator [Zola](https://github.com/getzola/zola).
The theme is derived from https://github.com/huhu/juice.

All colors can be edited in `templates/_variables.html`.

## Building

The entire page including the [mdbook](https://github.com/rust-lang/mdBook) and the nightly RustDoc can be built using `./build.sh`. The contents will be output in `./public`.

## Testing

To test the website locally you can execute `./build.sh` and serve the contents through
```
$ python -m http.server -d public
```
or any other similar http server.

To test on GitHub pages:
1. Change the source for Pages deployments in the Repository Settings > Pages to GitHub Actions.
2. This step is only needed if you don't put a custom domain in front of Pages: Change the `base_url` in `./config.toml` to `/<your-valence-fork-name>/` as GitHub Pages deployments for repositories aren't available at `/`.
3. Push to main or manually start the workflow in the Actions-tab on GitHub.

## Deployment

The deployment process is stated above in the Testing-section.

## Contribute Content to the Site

### Contribute to the FAQ / News

All contents are made in plain Markdown defined by [CommonMark](https://commonmark.org/).
A new FAQ / News entry can be added by simply creating a new Heading in `./content/faq.md` or `./content/news.md` respectively.

This can be tested locally by using `zola serve`.

### Contribute to the book

The book is made using mdbook and the source file lay in `./book/`.
See the [official documentation](https://rust-lang.github.io/mdBook/format/index.html) for how to create new pages.

This can be tested locally by using `mdbook serve`.

