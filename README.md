# asciinema

[![Build Status](https://github.com/asciinema/asciinema/actions/workflows/ci.yml/badge.svg)](https://github.com/asciinema/asciinema/actions/workflows/asciinema.yml)
[![license](http://img.shields.io/badge/license-GNU-blue.svg)](https://raw.githubusercontent.com/asciinema/asciinema/master/LICENSE)

__asciinema__ (aka asciinema CLI or asciinema recorder) is a command-line tool
for recording terminal sessions.

Unlike typical _screen_ recording software, which records visual output of a
screen into a heavyweight video files (`.mp4`, `.mov`), asciinema recorder runs
_inside a terminal_, capturing terminal session output into a lightweight
recording files in the
[asciicast](https://docs.asciinema.org/manual/asciicast/v2/) format (`.cast`).

The recordings can be replayed in a terminal, embedded on a web page with the
[asciinema player](https://docs.asciinema.org/manual/player/), or published to
an [asciinema server](https://docs.asciinema.org/manual/server/), such as
[asciinema.org](https://asciinema.org), for further sharing.

[![asciinema CLI
demo](https://asciinema.org/a/85R4jTtjKVRIYXTcKCNq0vzYH.svg)](https://asciinema.org/a/85R4jTtjKVRIYXTcKCNq0vzYH?autoplay=1)

Notable features:

* [recording](https://docs.asciinema.org/manual/cli/usage/#asciinema-rec-filename)
  and
  [replaying](https://docs.asciinema.org/manual/cli/usage/#asciinema-play-filename)
  of sessions inside a terminal,
* live streaming of terminal sessions, with local HTTP server mode, and a relay
  forwarding mode,
* [light-weight recording
  format](https://docs.asciinema.org/manual/asciicast/v2/), which is highly
  compressible (down to 15% of the original size e.g. with `zstd` or `gzip`),
* integration with [asciinema
  server](https://docs.asciinema.org/manual/server/), e.g.
  [asciinema.org](https://asciinema.org), for easy recording hosting.

To record a session run this command in your shell:

```sh
asciinema rec demo.cast
```

To stream a session via built-in HTTP server run:

```sh
asciinema stream --serve
```

To stream a session via a relay (asciinema server) run:

```sh
asciinema stream --relay
```

Check out the [Getting started
guide](https://docs.asciinema.org/getting-started/) for installation and usage
overview.

## Building

Building asciinema from source requires the [Rust](https://www.rust-lang.org/)
compiler (1.70 or later), and the [Cargo package
manager](https://doc.rust-lang.org/cargo/). If they are not available via your
system package manager then use [rustup](https://rustup.rs/).

To download the source code, build the asciinema binary, and install it in
`$HOME/.cargo/bin` run:

```sh
cargo install --locked --git https://github.com/asciinema/asciinema
```

Then, ensure `$HOME/.cargo/bin` is in your shell's `$PATH`.

Alternatively, you can manually download the source code and build the asciinema
binary with:

```sh
git clone https://github.com/asciinema/asciinema
cd asciinema
cargo build --release
```

This produces the binary in _release mode_ (`--release`) at
`target/release/asciinema`. You can just copy the binary to a directory in your
`$PATH`.

To generate man pages and shell completion files, set `ASCIINEMA_GEN_DIR` to the
path where these artifacts should be stored. For example:

```sh
ASCIINEMA_GEN_DIR=/foo cargo build --release
```

The above command will build the binary and place the man pages in `/foo/man/`,
and the shell completion files in the `/foo/completion/` directory.

The whole intallation proccess, include the man pages and shell completion.

```sh
git clone https://github.com/asciinema/asciinema
cd asciinema
cargo build --release
echo $PATH
sudo cp target/asciinema /usr/bin/
ls /usr/bin/ | grep ascii
ASCIINEMA_GEN_DIR=~/asciinema/ cargo build --release
ls man/ | wc -l
ls /usr/share/man/man1/
sudo cp man/* /usr/share/man/man1/
ls /usr/share/man/man1 | grep asci | wc -l
sudo mandb
man asciinema
ls completion/
ls /etc/bash_completion.d/
sudo cp completion/* /etc/bash_completion.d/
asciinema --help
```

> [!NOTE]
> Windows is currently not supported. _(See [#467](https://github.com/asciinema/asciinema/issues/467))_

## Development

This branch contains the next generation of the asciinema CLI, written in Rust
([about the
rewrite](https://discourse.asciinema.org/t/rust-rewrite-of-the-asciinema-cli/777)).
It is still in a work-in-progress stage, so if you wish to propose any code
changes, please first reach out to the team via
[forum](https://discourse.asciinema.org/),
[Matrix](https://matrix.to/#/#asciinema:matrix.org) or
[IRC](https://web.libera.chat/#asciinema).

The previous generation of the asciinema CLI, written in Python, can be found in
the `main` branch.

## Donations

Sustainability of asciinema development relies on donations and sponsorships.

Please help the software project you use and love. Become a
[supporter](https://docs.asciinema.org/donations/#individuals) or a [corporate
sponsor](https://docs.asciinema.org/donations/#corporate-sponsorship).

asciinema is sponsored by:

- [Brightbox](https://www.brightbox.com/)
- [DataDog](https://datadoghq.com/)

## Consulting

If you're interested in integration or customization of asciinema to suit your
needs, check [asciinema consulting
services](https://docs.asciinema.org/consulting/).

## License

© 2011 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See [LICENSE](./LICENSE) file
for details.
