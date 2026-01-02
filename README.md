# Mucap

Retrospective MIDI Recording Plugin

![Mucap Screenshot](/docs/img/mucap.png)

## Features

* Retrospective MIDI Recording
* CLAP, VST3, Standalone
* Sample accurate recording
* Records all basic MIDI events on all channels
* Bar markers used for selection snapping

## Installation

### NixOS / Flakes

Add the Mucap flake to your flake inputs. Make sure that you override the `nixpkgs` input to follow the Nixpkgs instance that you use for all your audio packages. Otherwise, you will risk library incompatibilities.

```
mucap = {
  url = "github:polygon/mucap";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Now you can add the package to your system, e.g.:

```
environment.systemPackages = [ mucap.packages.${system}.mucap ];
```

If not already done, make sure the VST3 and CLAP paths are exported to `/run/current-system/sw`:

```
environment.pathsToLink = [ "/lib/vst3" "/lib/clap" ];
```

If you are a user of [audio.nix](https://github.com/polygon/audio.nix), mucap is also re-exported there.

### Debian / Ubuntu based

Get the DEB archive from the [latest release](https://github.com/polygon/mucap/releases/latest) and install it, e.g.:

```
sudo apt install ./mucap-0.9.0-amd64.deb
```

## Usage

Add Mucap to a track that receives MIDI (or use Jack to connect a MIDI input if you run standalone). It records all MIDI events and draws notes in the UI. By default, the program will keep following the playhead.

You can zoom the canvas using <kbd>VScroll</kbd> and pan using <kbd>HScroll</kbd>. For users without horizontal scrolling, use <kbd>Shift</kbd> + <kbd>VScroll</kbd> to pan.

Select a range of MIDI events by pressing and holding <kbd>LMouse</kbd>, drag the cursor to select the range and release <kbd>LMouse</kbd> to complete the selection. This generates a MIDI file in your tmp folder and puts a reference to it in the clipboard. Select in your DAW where you want the MIDI to go and paste.

If you run inside a DAW and the transport plays, Mucap captures the locations of bars and will snap to them when selecting. To override snapping, hold <kbd>Shift</kbd> while selecting.

After 30 seconds of inactivity, Mucap will resume following the playhead.

## Operational peculiarities

Detailed information about program behaviors you may find useful.

### BPM and time stretching

Mucap records absolute note times relative to the plugin start having seconds as fundamental unit of time. MIDI files, in their most widely supported mode, use the length of a quarter note divided into 480 sub-ticks as their fundamental unit of time. Mucap needs to know the tempo of your playing in order to convert between these.

Mucap will always use the tempo it receives from the transport (120BPM for standalone operation) to achieve this conversion. If you play to a backing track running in your browser but have Mucap running inside the DAW, you need to make sure that your DAW has the correct BPM set or the clip length will be stretched (if DAW BPM is lower) or compressed (if DAW BPM is higher).

It is not required to have the tempo set correctly while recording, the conversion only happens after selection. So if you forget, it is not an issue.

Also, the relative positions of all events are correct. So you can stretch the clip at any time to match the track tempo if the tempo was wrong during export.

### Selection behavior

Mucap will export all MIDI events that happen inside the time selection. If the selection contains a partial note, its note start and note stop events are repeated at the start and end of selection.

### Multichannel

Mucap treats the MIDI channel like any other event property. Exported MIDI data will be a single track that contains all data of all channels. This should, in theory, make it work with MPE controllers, though that has not been tested so far.

It is not planned to offer special modes where different channels are handled differently. If you have this use-case, please launch one Mucap instance per channel and handle the MIDI routing in your DAW.

## Building

### Nix / NixOS environment

Mucap requires Flakes enabled in Nix. If you run nix-direnv, you can just allow the `.envrc` file, otherwise enter the devshell.

```sh
# With nix-direnv
direnv allow

# Normal devshell
nix develop
```

### Debian-based environment

Install the required dependencies (see also `distrib/Dockerfile`):

```sh
apt install \
    build-essential \
    git \
    rustup \
    pkg-config \
    libgl-dev \
    libx11-xcb-dev \
    libxcursor-dev \
    libasound2-dev \
    python3 \
    libjack-dev \
    libxcb-icccm4-dev \
    libxcb-dri2-0-dev
rustup toolchain install stable
```

### Building

Build using:

```
cargo xtask bundle mucap --release
```

The artifacts will be in `target/bundled`.

## Contributing

I want to make Mucap as accessible as possible. If you know packaging for distributions that are currently unsupported, I'd like to cooperate with you. I believe there is no reason why this should not also run under Windows or OSX, but alas, I could use some support there, too.

I am open for bug reports and feature requests, though especially the latter will kinda depend on personal interest. Pull Requests with new functionality are welcome but consider getting in contact prior if you plan something larger.
