use std::path::{Path, PathBuf};

use arboard::Clipboard;
use std::fs;
use midly::Smf;

fn main() {
    let data = fs::read("/home/jan/lala.mid").unwrap();
    let mut smf = Smf::parse(&data).unwrap();
    println!("{:?}", smf);
    return;
    smf.header.format = midly::Format::SingleTrack;
    smf.header.timing = midly::Timing::Timecode(midly::Fps::Fps25, 40);
    smf.save("/home/jan/test1.mid").unwrap();
    println!("{:?}", smf);
    let mut clippy = Clipboard::new().unwrap();
    println!("Clippy acquired");
    clippy.set().file_list(&[Path::new("/home/jan/test1.mid")]).unwrap();
    println!("Copied!");
    loop {}
}
