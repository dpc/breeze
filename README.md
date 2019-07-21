<p align="center">
  <a href="https://travis-ci.org/dpc/breeze">
      <img src="https://img.shields.io/travis/dpc/breeze/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://gitter.im/dpc/breeze">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
</p>

# `breeze` -  An innovative, modal, CLI-centric text/code editor

### Features & Goals

* Heavily inspired by [Kakoune](http://kakoune.org/)
* Modal & CLI-centric, but in a modern edition
    * `|`-shaped cursor
    * Kakoune-inspired editing experience
* Core library should compile to WebAssembly, so it can run everwhere, especially in the browser
* WebAssembly plugin support
    * Plugin-centric
    * Sandboxed, so they can't steal your Bitcoins!

I have recently switched to [kakoune](http://kakoune.org/) after years (decades?)
of using Vim. I think changing *action, movement* into *movement, action* is a
brilliant idea. I enjoy simplicity of Kakoune and I am generally quite happy using it.

However I have couple of ideas how Kakoune could be better and/or disagree with a couple
of things in it. So I decided to hack together my own code editor to demonstrate / try them.

## What is distinct about Breeze

Rust. Life is too short not to use Rust.

Terminals can do `|`-shaped cursors now, people! We don't have to use the blocky
cursor anymore! In Breeze `|` is the only cursor shape. Whole design assumes `|`-shaped
cursor. It feels more like a graphical text editor,
than traditional CLI ones. A fresh breeze in CLI terminal world.

Kakoune seem very Vim-golf-centric. In Breeze the philosophy is slightly different.
It doesn't matter to me in how many keystrokes one can perform certain editing operation.
What matter to me most is predictable, natural, easy to use modal text edition. Muscle
memory and rapid keypressing without having to pay much attention is what I am aiming for.


## Status and plans


![Breeze Screenshot](https://i.imgur.com/lzR8cME.png "Breeze screenshot")

Some stuff works, but still very, very early. And considering how little time I have,
it will probably stay this way for a long while. I might hack on it continously in the
future, or I might loose the motivation. I am happy to accept collaborators and help.

## Running

If you don't have Rust installed go to https://rustup.rs

Aftewards:

```
cargo run --release -- [file_path]
```
to run from source code, or

```
cargo install -f
```

to install.



## How to use (what works)

Breeze is modal. You are typically in the normal mode, enter insert mode with `i`, and leave it with `Esc`.
You know... just like in Vim or Kakoune.

Breeze has selections. Kind of like in Visual mode in Vim, just more automatic. If you've used Kakoune - they
are very much like in Kakoune.

What should work already:

* basic moves: `hjklwb%`
* numerical prefix for most of implemented stuff
* basic insert mode: `i`, `o`, `Esc`
* deletion: `d`, `c`
* copy&paste `y`, `p`, `P`
* `g` (followed by `h`, `j`, `k`, `l`)
* Ctrl-P (!!!)
* `'` - switch selection direction
* `<` and `>`
* line selection: `x`, `X`
* undo: `u` `U`
* basic commands: `:q`, `:e`, `:bn`, `:bp`, `:w`
