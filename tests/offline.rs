use float_eq::assert_float_eq;
use web_audio_api::context::BaseAudioContext;
use web_audio_api::context::OfflineAudioContext;
use web_audio_api::node::{
    AudioNode, AudioScheduledSourceNode, OscillatorNode, OscillatorOptions, OscillatorType,
};

const RENDER_QUANTUM_SIZE: usize = 128;

#[test]
fn test_offline_render() {
    const LENGTH: usize = 555;

    assert_ne!(LENGTH % RENDER_QUANTUM_SIZE, 0);

    let mut context = OfflineAudioContext::new(2, LENGTH, 44_100.);
    assert_eq!(context.length(), LENGTH);

    {
        let mut constant1 = context.create_constant_source();
        constant1.offset().set_value(2.);
        constant1.connect(&context.destination());

        let mut constant2 = context.create_constant_source();
        constant2.offset().set_value(-4.);
        constant2.connect(&context.destination());

        constant1.start();
        constant2.start();
    }

    let output = context.start_rendering_sync();
    assert_eq!(output.number_of_channels(), 2);
    assert_eq!(output.length(), LENGTH);

    assert_float_eq!(
        output.get_channel_data(0),
        &[-2.; LENGTH][..],
        abs_all <= 0.
    );
    assert_float_eq!(
        output.get_channel_data(1),
        &[-2.; LENGTH][..],
        abs_all <= 0.
    );
}

#[test]
fn test_start_stop() {
    let len = RENDER_QUANTUM_SIZE * 4;
    let sample_rate = 48000.;

    let mut context = OfflineAudioContext::new(1, len, sample_rate);
    assert_eq!(context.length(), len);

    {
        let opts = OscillatorOptions {
            type_: OscillatorType::Square,
            frequency: 0., // constant signal
            ..Default::default()
        };
        let mut osc = OscillatorNode::new(&context, opts);
        osc.connect(&context.destination());

        osc.start_at(128. / sample_rate as f64);
        osc.stop_at(128. * 3. / sample_rate as f64);
    }

    let output = context.start_rendering_sync();
    assert_eq!(output.number_of_channels(), 1);
    assert_eq!(output.length(), RENDER_QUANTUM_SIZE * 4);

    let channel_data = output.get_channel_data(0);

    // one chunk of silence, two chunks of signal, one chunk of silence
    let mut expected = vec![0.; RENDER_QUANTUM_SIZE];
    expected.append(&mut vec![1.; 2 * RENDER_QUANTUM_SIZE]);
    expected.append(&mut vec![0.; RENDER_QUANTUM_SIZE]);

    assert_float_eq!(channel_data, expected.as_slice(), abs_all <= 0.);
}

#[test]
fn test_delayed_constant_source() {
    let len = RENDER_QUANTUM_SIZE * 4;
    let sample_rate = 48000.;

    let mut context = OfflineAudioContext::new(1, len, sample_rate);
    assert_eq!(context.length(), len);

    {
        let delay = context.create_delay(1.);
        delay.delay_time().set_value(128. * 2. / sample_rate);
        delay.connect(&context.destination());

        let mut source = context.create_constant_source();
        source.connect(&delay);
        source.start();
    }

    let output = context.start_rendering_sync();
    assert_eq!(output.number_of_channels(), 1);
    assert_eq!(output.length(), RENDER_QUANTUM_SIZE * 4);

    let channel_data = output.get_channel_data(0);

    // two chunks of silence, two chunks of signal
    let mut expected = vec![0.; 2 * RENDER_QUANTUM_SIZE];
    expected.append(&mut vec![1.; 2 * RENDER_QUANTUM_SIZE]);

    assert_float_eq!(channel_data, expected.as_slice(), abs_all <= 0.00001);
}

#[test]
fn test_audio_param_graph() {
    let sample_rate = 48000.;
    let mut context = OfflineAudioContext::new(1, RENDER_QUANTUM_SIZE, sample_rate);
    {
        let gain = context.create_gain();
        gain.gain().set_value(0.5); // intrinsic value
        gain.connect(&context.destination());

        let mut source = context.create_constant_source();
        source.offset().set_value(0.8);
        source.connect(&gain);

        let mut param_input1 = context.create_constant_source();
        param_input1.offset().set_value(0.1);
        param_input1.connect(gain.gain());

        let mut param_input2 = context.create_constant_source();
        param_input2.offset().set_value(0.3);
        param_input2.connect(gain.gain());

        source.start();
        param_input1.start();
        param_input2.start();
    }

    let output = context.start_rendering_sync();
    assert_eq!(output.number_of_channels(), 1);
    assert_eq!(output.length(), RENDER_QUANTUM_SIZE);

    let channel_data = output.get_channel_data(0);

    // expect output = 0.8 (input) * ( 0.5 (intrinsic gain) + 0.4 (via 2 constant source input) )
    let expected = vec![0.8 * 0.9; RENDER_QUANTUM_SIZE];
    assert_eq!(channel_data, expected.as_slice());
}

#[test]
fn test_listener() {
    let sample_rate = 48000.;
    let mut context = OfflineAudioContext::new(1, RENDER_QUANTUM_SIZE, sample_rate);

    {
        let listener1 = context.listener();
        let listener2 = context.listener();
        listener1.position_x().set_value(1.);
        listener2.position_y().set_value(2.);
    }

    let listener = context.listener();
    let _ = context.start_rendering_sync();

    assert_float_eq!(listener.position_y().value(), 2., abs <= 0.);
    assert_float_eq!(listener.position_x().value(), 1., abs <= 0.);
}

#[test]
fn test_cycle() {
    let mut context = OfflineAudioContext::new(1, RENDER_QUANTUM_SIZE, 48000.);

    {
        let cycle1 = context.create_gain();
        cycle1.connect(&context.destination());

        let cycle2 = context.create_gain();
        cycle2.connect(&cycle1);

        // here we go
        cycle1.connect(&cycle2);

        let mut source_cycle = context.create_constant_source();
        source_cycle.offset().set_value(1.);
        source_cycle.connect(&cycle1);

        let mut other = context.create_constant_source();
        other.offset().set_value(2.);
        other.connect(&context.destination());

        source_cycle.start();
        other.start();
    }

    let output = context.start_rendering_sync();
    // cycle should be muted, and other source should be processed
    assert_float_eq!(
        output.get_channel_data(0),
        &[2.; RENDER_QUANTUM_SIZE][..],
        abs_all <= 0.
    );
}

#[test]
fn test_cycle_breaker() {
    let sample_rate = 48000.;
    let mut context = OfflineAudioContext::new(1, RENDER_QUANTUM_SIZE * 3, sample_rate);

    {
        let delay = context.create_delay(1. / sample_rate as f64);
        delay.delay_time().set_value(1. / sample_rate);
        delay.connect(&context.destination());

        // here we go
        delay.connect(&delay);

        let mut source = context.create_constant_source();
        source.offset().set_value(1.);
        source.connect(&delay);
        source.connect(&context.destination());

        source.start();
    }

    let output = context.start_rendering_sync();

    // not muted, and positive feedback cycle
    assert_float_eq!(
        output.get_channel_data(0)[..RENDER_QUANTUM_SIZE],
        &[1.; RENDER_QUANTUM_SIZE][..],
        abs_all <= 0.
    );
    assert_float_eq!(
        output.get_channel_data(0)[RENDER_QUANTUM_SIZE..2 * RENDER_QUANTUM_SIZE],
        &[2.; RENDER_QUANTUM_SIZE][..],
        abs_all <= 0.
    );
    assert_float_eq!(
        output.get_channel_data(0)[2 * RENDER_QUANTUM_SIZE..3 * RENDER_QUANTUM_SIZE],
        &[3.; RENDER_QUANTUM_SIZE][..],
        abs_all <= 0.
    );
}

#[test]
fn test_repeated_connect_with_identical_termini_is_a_single_connection() {
    // Spec: "There can only be one connection between a given output of one
    // specific node and a given input of another specific node. Multiple
    // connections with the same termini are ignored."
    let mut context = OfflineAudioContext::new(1, RENDER_QUANTUM_SIZE, 44_100.);

    let mut constant = context.create_constant_source();
    constant.offset().set_value(1.);
    constant.connect(&context.destination());
    constant.connect(&context.destination()); // must be ignored
    constant.connect(&context.destination()); // must be ignored
    constant.start();

    let output = context.start_rendering_sync();
    let samples = output.get_channel_data(0);
    assert_float_eq!(samples[64], 1., abs <= 0.);

    // after an explicit disconnect the connection can be re-established
    constant.disconnect_dest(&context.destination());
    constant.connect(&context.destination());
}

// WPT webaudio retrospective-setValueAtTime.html, transcribed: an automation time in the past must
// clamp to the current time, so a ramp anchored on it starts *now* rather than appearing to have
// been running since that past time. Two identical graphs, the only difference being the startTime
// passed to setValueAtTime; every rendered sample must match.
#[test]
fn test_retrospective_set_value_at_time_clamps_to_now() {
    use web_audio_api::context::{BaseAudioContext, OfflineAudioContext};
    use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};

    let length = 16384;
    let sample_rate = 16384.;
    let suspend_frame = 128;

    let mut context = OfflineAudioContext::new(2, length, sample_rate);
    let mut source = context.create_constant_source();
    let test = context.create_gain();
    let reference = context.create_gain();
    source.connect(&test);
    source.connect(&reference);

    let merger = context.create_channel_merger(2);
    test.connect_from_output_to_input(&merger, 0, 0);
    reference.connect_from_output_to_input(&merger, 0, 1);
    merger.connect(&context.destination());

    let ramp_end = (length - suspend_frame) as f32;
    let ramp_end_seconds = length as f64 / sample_rate as f64;

    let test_gain = test.gain().clone();
    let reference_gain = reference.gain().clone();
    context.suspend_sync(suspend_frame as f64 / sample_rate as f64, move |ctx| {
        let now = ctx.current_time();
        test_gain.set_value_at_time(0., 0.5 * now); // in the past
        test_gain.linear_ramp_to_value_at_time(ramp_end, ramp_end_seconds);
        reference_gain.set_value_at_time(0., now);
        reference_gain.linear_ramp_to_value_at_time(ramp_end, ramp_end_seconds);
    });

    source.start();
    let buffer = context.start_rendering_sync();
    let actual = buffer.get_channel_data(0);
    let expected = buffer.get_channel_data(1);

    for i in suspend_frame..length {
        assert!(
            (actual[i] - expected[i]).abs() < 1e-6,
            "sample {i}: test {} != reference {}",
            actual[i],
            expected[i]
        );
    }
}
