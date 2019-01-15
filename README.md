<p align="center">
  <a href="https://travis-ci.org/dpc/breeze">
      <img src="https://img.shields.io/travis/dpc/breeze/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://gitter.im/dpc/breeze">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
</p>



# `breeze` -  An experimental, kakoune-inspired CLI-centric text/code editor with |-shaped cursor (in Rust)

I have recently switched to [kakoune](http://kakoune.org/) after years (decades?)
of using Vim. I think changing *action, movement* into *movement, action* is a
brilliant idea. I also enjoy simplicity of Kakoune and am generally quite happy using it.

However I have couple of ideas how how Kakoune could be better/disagree with couple
of things in it.

So I decided to hack together my own code editor to demonstrate / try them.

## What is distinct about Breeze

Rust. Life is to short not to use Rust.

Terminals can do `|`-shaped cursors now, people! We don't have to use the blocky
cursor anymore! Am I really the only one to figure it out?
In Breeze `|` is the only cursor shape. It feels more like a graphical text editor,
than traditional CLI ones.

Kakoune seem very Vim-golf, centric. In Breeze the philosophy is slightly different.
It doesn't matter to me in how many keystrokes one can perform certain editing operation.
What matter instead is predictable, natural, easy to use modal text edition. Muscle
memory and rapid keypressing without having to pay much attention is what I am aiming for.


## Status and plans


![Breeze Screenshot](https://i.imgur.com/lzR8cME.png "Breeze screenshot")

Some stuff works, but still very, very early. And considering how little time I have,
it will probably stay this way for a long while. I might hack on it continously in the
future, or I might loose the motivation. If you feel like you are interested in helping,
I am happy to accept collaborators.
