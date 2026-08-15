//! Spatialization/Panning primitives
//!
//! Required for panning algorithm, distance and cone effects of panner nodes

use crate::context::{AudioContextRegistration, BaseAudioContext};
use crate::node::{
    AudioNode, AudioNodeOptions, ChannelConfig, ChannelCountMode, ChannelInterpretation,
};
use crate::param::{AudioParam, AudioParamDescriptor, AudioParamInner, AutomationRate};
use crate::render::{
    AudioParamValues, AudioProcessor, AudioRenderQuantum, AudioWorkletGlobalScope,
};

use std::f32::consts::PI;
use std::sync::OnceLock;

/// AudioParam settings for the cartesian coordinates
pub(crate) const PARAM_OPTS: AudioParamDescriptor = AudioParamDescriptor {
    name: String::new(),
    min_value: f32::MIN,
    max_value: f32::MAX,
    default_value: 0.,
    automation_rate: AutomationRate::A,
};

/// Represents the position and orientation of the person listening to the audio scene
///
/// All [`PannerNode`](crate::node::PannerNode) objects spatialize in relation to the [BaseAudioContext's](crate::context::BaseAudioContext) listener.
///
/// # Usage
///
/// For example usage, check the [`PannerNode`](crate::node::PannerNode) docs.
#[derive(Debug)]
pub struct AudioListener {
    pub(crate) position_x: AudioParam,
    pub(crate) position_y: AudioParam,
    pub(crate) position_z: AudioParam,
    pub(crate) forward_x: AudioParam,
    pub(crate) forward_y: AudioParam,
    pub(crate) forward_z: AudioParam,
    pub(crate) up_x: AudioParam,
    pub(crate) up_y: AudioParam,
    pub(crate) up_z: AudioParam,
}

impl AudioListener {
    pub fn position_x(&self) -> &AudioParam {
        &self.position_x
    }
    pub fn position_y(&self) -> &AudioParam {
        &self.position_y
    }
    pub fn position_z(&self) -> &AudioParam {
        &self.position_z
    }
    pub fn forward_x(&self) -> &AudioParam {
        &self.forward_x
    }
    pub fn forward_y(&self) -> &AudioParam {
        &self.forward_y
    }
    pub fn forward_z(&self) -> &AudioParam {
        &self.forward_z
    }
    pub fn up_x(&self) -> &AudioParam {
        &self.up_x
    }
    pub fn up_y(&self) -> &AudioParam {
        &self.up_y
    }
    pub fn up_z(&self) -> &AudioParam {
        &self.up_z
    }
}

/// Wrapper for the [`AudioListener`] so it can be placed in the audio graph.
///
/// This node has no input, but takes the position/orientation AudioParams and copies them into the
/// 9 outputs. The outputs are connected to the PannerNodes (via an AudioParam).
///
/// The AudioListener is always connected to the AudioDestinationNode so at each
/// render quantum its positions are recalculated.
pub(crate) struct AudioListenerNode {
    registration: AudioContextRegistration,
    fields: AudioListener,
}

impl AudioNode for AudioListenerNode {
    fn registration(&self) -> &AudioContextRegistration {
        &self.registration
    }

    fn channel_config(&self) -> &ChannelConfig {
        static INSTANCE: OnceLock<ChannelConfig> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            AudioNodeOptions {
                channel_count: 1,
                channel_count_mode: ChannelCountMode::Explicit,
                channel_interpretation: ChannelInterpretation::Discrete,
            }
            .into()
        })
    }

    fn number_of_inputs(&self) -> usize {
        0
    }

    fn number_of_outputs(&self) -> usize {
        9 // return all audio params as output
    }

    fn set_channel_count(&self, _v: usize) {
        panic!("NotSupportedError - AudioListenerNode has channel count constraints");
    }
    fn set_channel_count_mode(&self, _v: ChannelCountMode) {
        panic!("NotSupportedError - AudioListenerNode has channel count mode constraints");
    }
    fn set_channel_interpretation(&self, _v: ChannelInterpretation) {
        panic!("NotSupportedError - AudioListenerNode has channel interpretation constraints");
    }
}

impl AudioListenerNode {
    pub fn new<C: BaseAudioContext>(context: &C) -> Self {
        context.base().register(move |registration| {
            let forward_z_opts = AudioParamDescriptor {
                default_value: -1.,
                ..PARAM_OPTS
            };
            let up_y_opts = AudioParamDescriptor {
                default_value: 1.,
                ..PARAM_OPTS
            };

            let (p1, _v1) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p2, _v2) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p3, _v3) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p4, _v4) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p5, _v5) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p6, _v6) = context.create_audio_param(forward_z_opts, &registration);
            let (p7, _v7) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p8, _v8) = context.create_audio_param(up_y_opts, &registration);
            let (p9, _v9) = context.create_audio_param(PARAM_OPTS, &registration);

            let node = Self {
                registration,
                fields: AudioListener {
                    position_x: p1,
                    position_y: p2,
                    position_z: p3,
                    forward_x: p4,
                    forward_y: p5,
                    forward_z: p6,
                    up_x: p7,
                    up_y: p8,
                    up_z: p9,
                },
            };
            let proc = ListenerRenderer {};

            (node, Box::new(proc))
        })
    }

    pub fn into_fields(self) -> AudioListener {
        self.fields
    }
}

struct ListenerRenderer {}

impl AudioProcessor for ListenerRenderer {
    fn process(
        &mut self,
        _inputs: &[AudioRenderQuantum],
        _outputs: &mut [AudioRenderQuantum],
        _params: AudioParamValues<'_>,
        _scope: &AudioWorkletGlobalScope,
    ) -> bool {
        // do nothing, the Listener is just here to make sure the position/forward/up params render in order

        true // never drop
    }
}

/// Data holder for the BaseAudioContext so it can reconstruct the AudioListener on request
pub(crate) struct AudioListenerParams {
    pub position_x: AudioParamInner,
    pub position_y: AudioParamInner,
    pub position_z: AudioParamInner,
    pub forward_x: AudioParamInner,
    pub forward_y: AudioParamInner,
    pub forward_z: AudioParamInner,
    pub up_x: AudioParamInner,
    pub up_y: AudioParamInner,
    pub up_z: AudioParamInner,
}

use vecmath::{vec3_dot, vec3_len, vec3_normalized, vec3_square_len, vec3_sub, Vector3};

/// Direction to source position measured from listener in 3D
pub fn azimuth_and_elevation(
    source_position: Vector3<f32>,
    listener_position: Vector3<f32>,
    listener_forward: Vector3<f32>,
    listener_up: Vector3<f32>,
) -> (f32, f32) {
    // Computed in f64 throughout, narrowed only on return.
    //
    // The chain is normalize -> dot -> acos -> degrees, and `acos` loses a lot of precision in f32
    // near +-1, which is exactly where a source in front of the listener sits. The resulting angular
    // error feeds straight into the equal-power gain curve; WPT's panner tests allow 1.16e-6 on that
    // curve and the f32 chain overshot it. Positions and orientations stay f32 at the API boundary.
    type V = (f64, f64, f64);
    let v = |a: Vector3<f32>| -> V { (f64::from(a[0]), f64::from(a[1]), f64::from(a[2])) };
    let sub = |a: V, b: V| -> V { (a.0 - b.0, a.1 - b.1, a.2 - b.2) };
    let dot = |a: V, b: V| -> f64 { a.0.mul_add(b.0, a.1.mul_add(b.1, a.2 * b.2)) };
    let cross = |a: V, b: V| -> V {
        (
            a.1 * b.2 - a.2 * b.1,
            a.2 * b.0 - a.0 * b.2,
            a.0 * b.1 - a.1 * b.0,
        )
    };
    let square_len = |a: V| -> f64 { dot(a, a) };
    let scale = |a: V, k: f64| -> V { (a.0 * k, a.1 * k, a.2 * k) };
    let normalized = |a: V| -> V {
        let len = square_len(a).sqrt();
        if len == 0. {
            a
        } else {
            scale(a, 1. / len)
        }
    };

    let relative_pos = sub(v(source_position), v(listener_position));

    // Handle degenerate case if source and listener are at the same point.
    if square_len(relative_pos) <= f64::from(f32::MIN_POSITIVE) {
        return (0., 0.);
    }

    // Calculate the source-listener vector.
    let source_listener = normalized(relative_pos);

    // Align axes.
    let listener_right = cross(v(listener_forward), v(listener_up));

    if square_len(listener_right) == 0. {
        // Handle the case where listener's 'up' and 'forward' vectors are linearly dependent, in
        // which case 'right' cannot be determined
        return (0., 0.);
    }

    // Determine a unit vector orthogonal to listener's right, forward
    let listener_right_norm = normalized(listener_right);
    let listener_forward_norm = normalized(v(listener_forward));
    let up = cross(listener_right_norm, listener_forward_norm);

    // Determine elevation first
    let mut elevation =
        90. - 180. * dot(source_listener, up).clamp(-1., 1.).acos() / std::f64::consts::PI;
    if elevation > 90. {
        elevation = 180. - elevation;
    } else if elevation < -90. {
        elevation = -180. - elevation;
    }

    let up_projection = dot(source_listener, up);
    let projected_source = sub(source_listener, scale(up, up_projection));

    // this case is not handled by the spec, so I stole the solution from
    // https://hg.mozilla.org/mozilla-central/rev/1100a5bc013b541c635bc42bd753531e95c952e4
    if square_len(projected_source) == 0. {
        return (0., elevation as f32);
    }
    let projected_source = normalized(projected_source);

    // `acos` is only defined on [-1, 1]; rounding can push a dot product of two unit vectors a hair
    // outside, which yields NaN and poisons the whole gain chain.
    let mut azimuth = 180.
        * dot(projected_source, listener_right_norm)
            .clamp(-1., 1.)
            .acos()
        / std::f64::consts::PI;

    // Source in front or behind the listener.
    let front_back = dot(projected_source, listener_forward_norm);
    if front_back < 0. {
        azimuth = 360. - azimuth;
    }

    // Make azimuth relative to "forward" and not "right" listener vector.
    #[allow(clippy::manual_range_contains)]
    if azimuth >= 0. && azimuth <= 270. {
        azimuth = 90. - azimuth;
    } else {
        azimuth = 450. - azimuth;
    }

    (azimuth as f32, elevation as f32)
}

/// Distance between two points in 3D
pub fn distance(source_position: Vector3<f32>, listener_position: Vector3<f32>) -> f32 {
    vec3_len(vec3_sub(source_position, listener_position))
}

/// Angle between two vectors in 3D
pub fn angle(
    source_position: Vector3<f32>,
    source_orientation: Vector3<f32>,
    listener_position: Vector3<f32>,
) -> f32 {
    // handle edge case of missing source orientation
    if vec3_square_len(source_orientation) == 0. {
        return 0.;
    }
    let normalized_source_orientation = vec3_normalized(source_orientation);

    let relative_pos = vec3_sub(source_position, listener_position);
    // Handle degenerate case if source and listener are at the same point.
    if vec3_square_len(relative_pos) <= f32::MIN_POSITIVE {
        return 0.;
    }
    // Calculate the source-listener vector.
    let source_listener = vec3_normalized(relative_pos);

    let angle = 180. * vec3_dot(source_listener, normalized_source_orientation).acos() / PI;
    angle.abs()
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;

    // listener coordinates/directions
    const LP: [f32; 3] = [0., 0., 0.];
    const LF: [f32; 3] = [0., 0., -1.];
    const LU: [f32; 3] = [0., 1., 0.];

    #[test]
    fn azimuth_elevation_equal_pos() {
        let pos = [0., 0., 0.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);

        assert_float_eq!(azimuth, 0., abs <= 0.);
        assert_float_eq!(elevation, 0., abs <= 0.);
    }

    #[test]
    fn azimuth_elevation_horizontal_plane() {
        // horizontal plane is spanned by x-z axes

        let pos = [10., 0., 0.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, 90., abs <= 0.001);
        assert_float_eq!(elevation, 0., abs <= 0.);

        let pos = [-10., 0., 0.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, -90., abs <= 0.001);
        assert_float_eq!(elevation, 0., abs <= 0.);

        let pos = [10., 0., -10.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, 45., abs <= 0.001);
        assert_float_eq!(elevation, 0., abs <= 0.);

        let pos = [-10., 0., -10.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, -45., abs <= 0.001);
        assert_float_eq!(elevation, 0., abs <= 0.);
    }

    #[test]
    fn azimuth_elevation_vertical() {
        let pos = [0., -10., 0.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, 0., abs <= 0.001);
        assert_float_eq!(elevation, -90., abs <= 0.001);

        let pos = [0., 10., 0.];
        let (azimuth, elevation) = azimuth_and_elevation(pos, LP, LF, LU);
        assert_float_eq!(azimuth, 0., abs <= 0.001);
        assert_float_eq!(elevation, 90., abs <= 0.001);
    }

    #[test]
    fn angle_equal_pos() {
        let pos = [0., 0., 0.];
        let orientation = [1., 0., 0.];
        let angle = angle(pos, orientation, LP);

        assert_float_eq!(angle, 0., abs <= 0.);
    }

    #[test]
    fn angle_no_orientation() {
        let pos = [10., 0., 0.];
        let orientation = [0., 0., 0.];
        let angle = angle(pos, orientation, LP);

        assert_float_eq!(angle, 0., abs <= 0.);
    }

    #[test]
    fn test_angle() {
        let pos = [1., 0., 0.];
        let orientation = [0., 1., 0.];
        let angle = angle(pos, orientation, LP);

        assert_float_eq!(angle, 90., abs <= 0.);
    }

    #[test]
    fn test_angle_abs_value() {
        let pos = [1., 0., 0.];
        let orientation = [0., -1., 0.];
        let angle = angle(pos, orientation, LP);

        assert_float_eq!(angle, 90., abs <= 0.);
    }
}
