<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Client-Side Rendered (CSR) App Starter Template

This is a template for use with the [Leptos][Leptos] web framework using the [Trunk][Trunk] tool to compile and serve your app in development.

## Creating your repo from the template

This template requires you to have `cargo-generate` installed. You can install it with

```sh
cargo install cargo-generate
```


To set up your project with this template, run

```sh
cargo generate --git https://github.com/leptos-community/start-csr
```

to generate your new project, then

```sh
cd start-csr
```

to go to your newly created project.

By default, this template uses Rust `nightly` and requires that you've installed the `wasm` compilation target for your toolchain.


Sass and Tailwind are also supported by the Trunk build tool, but are optional additions: [see here for more info on how to set those up with Trunk][Trunk-instructions].


If you don't have Rust nightly, you can install it with
```sh
rustup toolchain install nightly --allow-downgrade
```

You can add the `wasm` compilation target to rust using
```sh
rustup target add wasm32-unknown-unknown
```


## Developing your Leptos CSR project

To develop your Leptos CSR project, running

```sh
trunk serve --port 3000 --open
```

will open your app in your default browser at `http://localhost:3000`.


## Deploying your Leptos CSR project

To build a Leptos CSR app for release, use the command

```sh
trunk build --release
```

This will output the files necessary to run your app into the `dist` folder; you can then use any static site host to serve these files.

For further information about hosting Leptos CSR apps, please refer to [the Leptos Book chapter on deployment available here][deploy-csr].


[Leptos]: https://github.com/leptos-rs/leptos

[Trunk]: https://github.com/trunk-rs/trunk
[Trunk-instructions]: https://trunkrs.dev/assets/

[deploy-csr]: https://book.leptos.dev/deployment/csr.html