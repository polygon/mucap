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

If you haven't alredy done, you probbly want to make sure that the VST3 and CLAP paths are exported to `/run/current-system/sw`:

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

Add Mucap to a track that receives MIDI (or use Jack to connect a MIDI input if you run standalone) and it will record all MIDI events received and draw the notes in the main window. By default, the program will keep following the playhead.

You can zoom the canvas using <kbd>VScroll</kbd> and pan using <kbd>HScroll</kbd>. For users without horizontal scrolling, use <kbd>Shift</kbd> + <kbd>VScroll</kbd> to pan.

Select a range of MIDI events by pressing and holding <kbd>LMouse</kbd>, drag the cursor to select the range and complete selection by releasing <kbd>LMouse</kbd>. This generates a MIDI file in your tmp folder and puts a reference to it in the clipboard. Select in your DAW where you want the MIDI to go and paste.

If you run inside a DAW and the transport plays, Mucap captures the locations of bars and will snap to them when selecting. To override snapping, hold <kbd>Shift</kbd> while selecting.

## Operational peculiarities

Detailed information about program behaviors you may find useful. Some of these were design choices that needed to be made.

### BPM and time stretching

Mucap records absolute note times relative to the plugin start having seconds as fundamental unit of time. MIDI files, in their most widely supported mode, use the length of a quarter note divided into 480 sub-ticks as their fundamental unit of time. Mucap needs to know the tempo of your playing in order to convert between these.

Mucap will always use the tempo it receives from the transport (120BPM for standalone operation) to achieve this conversion. If you, e.g., play to a backing track running in your browser but have Mucap running inside the DAW, you need to make sure that your DAW has the correct BPM set or the clip length will be stretched (if DAW BPM is lower) or compressed (if DAW BPM is higher).

It is NOT needed to have the tempo set correctly while recording, the conversion only happens after selection. So if you forget, it is not an issue.

Also, the relative positions of all events are correct. So you can stretch the clip at any time to match the track tempo if you forgot while Mucap was running.

### Selection behavior

Mucap will export all MIDI events that happen inside the time selection, including the start and end times. If the selection contains a partial note, its note start and note stop events are repeated at the start and end of selection.

Partially selected notes that end, after converting to 480 PPQN (pulses per quarter note) time-base would start end end on tick 0, creating a note of zero length, will be discarded as these would create invalid ghost notes of much longer duration in Bitwig Studio.

### Multichannel

Mucap treats the MIDI channel like any other event property. Exportet MIDI data will be single track that contains all data of all channels. This should, in theory, make it work with MPE controllers, tho that has not been tested so far.

It is not planned to offer special modes where different channels are handled differently. If you have this use-case, please launch one Mucap instance per channel and handle the MIDI routing in your DAW.
