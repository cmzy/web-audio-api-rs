//! Regression: a MediaStreamAudioDestinationNode whose control handle is dropped must keep
//! feeding its MediaStream.

use web_audio_api::context::{AudioContext, AudioContextOptions, BaseAudioContext};
use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};

#[test]
fn stream_survives_dropping_the_node_handle() {
    let context = AudioContext::new(AudioContextOptions {
        sink_id: "none".into(),
        ..AudioContextOptions::default()
    });

    let dest = context.create_media_stream_destination();
    let mut osc = context.create_oscillator();
    osc.connect(&dest);
    osc.start();

    let stream = dest.stream().clone();
    let track = stream.get_tracks().first().cloned().unwrap();
    let mut chunks = track.iter();

    // The consumer holds the stream, but nothing holds the node - the idiomatic shape, and the
    // only one available to bindings for garbage collected languages, where the handle is dropped
    // whenever the GC decides the script can no longer reach the node.
    drop(dest);

    std::thread::sleep(std::time::Duration::from_millis(200));

    for i in 0..20 {
        match chunks.next() {
            Some(Ok(_)) => {}
            Some(Err(e)) => panic!("stream broke {i} chunks after dropping the node handle: {e}"),
            None => panic!("stream ended {i} chunks after dropping the node handle"),
        }
    }
}
