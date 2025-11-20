> [!WARNING]
> This tool is unfinished and currently NOT fit for any kind of use. Essential features are still missing.

# MuCap

Saves your MIDI while you forgot to press record.

## Design / Technical Challenges

Main goal is to use this from Bitwig, tho it should work with every DAW that supports VST3/CLAP plugins as well as run standalone, albeit the latter not being a huge priority beyond testing.

The main challenges identified:

1. Need to build a plugin that can run in a DAW, receives MIDI, has a GUI
2. Need to capture the MIDI, reconstruct notes for drawing, select ranges of MIDI data for later export
3. Figure out a way to take the MIDI data and get it back into the DAW, ideally with a drag & drop gesture

### Plugin framework

* Interfacing with DAWs is neccessary but technically quite complex
* UI integration is also challenging
* Framework with most heavy lifting done is imperative
* Only nih-plug seems to fit the bill

* [x] Implemented test plugin: simple enough
* [x] Ensure that MIDI data can be received and processed

**Decision**: Go with NIH-plug

### Timing

* MIDI capturing needs to be impeccably precise, otherwise there is no point to this plugin at all
* Timing of MIDI events needs to be captured with as high precision as possible

Available options:

* Use timing information from transport: Not good, transport is not always running and might not even be available at all, also, we should be independent from time jumps in the transport
* Use internal clock to generate own timestamps: Probably good enough, but relies on assumptions and might be jittery
* Counting samples: Regular sample processing seems to be the bread and butter of plugins, MIDI events are given with a relativ sample offset in the current buffer, and counting buffers should allow for very precise timing.

* [x] Tested timekeeping with samples is persistent over transport operations

**Decision:** Go with sample-counting as timing source

### UI

* NIH-Plug supports three graphical frameworks as integrations:
    * egui - Known from Bevy, immediate mode - I'm not a fan of intermediate mode
    * iced - Looks good, might use in the future
    * vizia - Clicked the best

I don't have really clear requirements here. I will exclude egui because I found immediate mode GUIs to be a bit of a hassle when working with Ratatui, even though egui might be different. Not want to spend too much time here so I am using the one that looks best when looking at the NIH-Plug examples

* [x] Implemented small application that demonstrates drag & drop from GUI to outside window

**Decision:** Use vizia, purely based on first impressions

### Get MIDI data from plugin back to DAW

* MIDI data in clipboard seems to be quite proprietary
    * Clipboard contents would change when copying MIDI data in Bitwig, but I was unable to even read them out
    * Unable to copy and paste from Bitwig to Ardour and the other way around
    * Either Bitwig is doing it's own thing, or possibly everyone
* Other plugins (NeuralNote) seem to create temporary MIDI files and just move the file reference around
    * Not my preferred way, but it seems to work relatively universally
* `arboard` seems like a good clipboard library for Rust that has support to a FileList style content

* [x] Implement test application with arboard, that puts a file reference to a .mid-file into the clipbaord - Pasting into Bitwig works successfully

**Decision:** Go with `arboard` and temporary MIDI files, but also keep an eye open for a solution without temporary files.
