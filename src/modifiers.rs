use bevy::prelude::*;

use crate::{
    asset::{InitLayout, RenderLayout, UpdateLayout},
    gradient::Gradient,
    ToWgslString, Value,
};

/// Maximum number of components in the force field.
pub const FFNUM: usize = 16;

/// Trait to customize the initializing of newly spawned particles.
pub trait InitModifier {
    /// Apply the modifier to the init layout of the effect instance.
    fn apply(&self, init_layout: &mut InitLayout);
}

/// Trait to customize the updating of existing particles each frame.
pub trait UpdateModifier {
    /// Apply the modifier to the update layout of the effect instance.
    fn apply(&self, update_layout: &mut UpdateLayout);
}

/// Trait to customize the rendering of alive particles each frame.
pub trait RenderModifier {
    /// Apply the modifier to the render layout of the effect instance.
    fn apply(&self, render_layout: &mut RenderLayout);
}

/// The dimension of a shape to consider.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShapeDimension {
    /// Consider the surface of the shape only.
    Surface,
    /// Consider the entire shape volume.
    Volume,
}

impl Default for ShapeDimension {
    fn default() -> Self {
        ShapeDimension::Surface
    }
}

/// The direction and intensity of the speed vector for particles
#[derive(Clone, Copy, PartialEq)]
pub enum SpeedVector {
    /// The particle will get ejected along the face normal
    Normal(Value<f32>),
    /// The particle will get ejected away from the shape's center
    Radial(Value<f32>),
    /// The particle will get ejected in the given (x, y, z) direction (local space)
    Local(Value<f32>, Value<f32>, Value<f32>),
    /// The particle will get ejected in the given (x, y, z) direction (world space)
    World(Value<f32>, Value<f32>, Value<f32>),
}

impl Default for SpeedVector {
    fn default() -> Self {
        Self::Radial(Value::Single(1.0))
    }
}

/// An initialization modifier spawning particles on a circle/disc.
#[derive(Default, Clone, Copy)]
pub struct PositionCircleModifier {
    /// The circle center, relative to the emitter position.
    pub center: Vec3,
    /// The circle rotation.
    /// Set this to `Quat::from_rotation_x(PI / 2.)` for a 2D game.
    pub rotation: Quat,
    /// The circle radius.
    pub radius: f32,
    /// The speed of the particles on spawn.
    /// In this case, SpeedVector::Radial has the same effect than SpeedVector::Normal.
    pub speed: SpeedVector,
    /// The shape dimension to spawn from.
    pub dimension: ShapeDimension,
}

impl InitModifier for PositionCircleModifier {
    fn apply(&self, init_layout: &mut InitLayout) {
        let tangent = self.rotation * Vec3::X;
        let bitangent = self.rotation * Vec3::Z;

        let radius_code = match self.dimension {
            ShapeDimension::Surface => {
                // Constant radius
                format!("let r = {};", self.radius.to_wgsl_string())
            }
            ShapeDimension::Volume => {
                // Radius uniformly distributed in [0:1], then square-rooted
                // to account for the increased perimeter covered by increased radii.
                format!("let r = sqrt(rand()) * {};", self.radius.to_wgsl_string())
            }
        };

        let speed_code = match self.speed {
            SpeedVector::Normal(speed) | SpeedVector::Radial(speed) => {
                format!("ret.vel = dir * ({});", speed.to_wgsl_string())
            }
            SpeedVector::Local(speed_x, speed_y, speed_z) => {
                let rotation: Vec4 = self.rotation.into();
                format!(
                    "
    let rot = {};
    ret.vel = rotate_point(vec3<f32>({}, {}, {}), rot);
                        ",
                    rotation.to_wgsl_string(),
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
            SpeedVector::World(speed_x, speed_y, speed_z) => {
                format!(
                    "ret.vel = vec3<f32>({}, {}, {});",
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
        };

        init_layout.position_code = format!(
            r##"
    // >>> [PositionCircleModifier]
    // Circle center
    let c = {};
    // Circle basis
    let tangent = {};
    let bitangent = {};
    // Circle radius
    {}
    // Spawn random point on/in circle
    let theta = rand() * tau;
    let dir = tangent * cos(theta) + bitangent * sin(theta);
    ret.pos = c + r * dir;
    // Speed code
    {}
    // <<< [PositionCircleModifier]
            "##,
            self.center.to_wgsl_string(),
            tangent.to_wgsl_string(),
            bitangent.to_wgsl_string(),
            radius_code,
            speed_code,
        );
    }
}

/// An initialization modifier spawning particles on a sphere.
#[derive(Default, Clone, Copy)]
pub struct PositionSphereModifier {
    /// The sphere center, relative to the emitter position.
    pub center: Vec3,
    /// The sphere radius.
    pub radius: f32,
    /// The sphere alignment relative to the world.
    /// Used for SpeedVector::Local and SpeedVector::World.
    pub rotation: Quat,
    /// The speed of the particles on spawn.
    /// In this case, SpeedVector::Radial has the same effect than SpeedVector::Normal.
    pub speed: SpeedVector,
    /// The shape dimension to spawn from.
    pub dimension: ShapeDimension,
}

impl InitModifier for PositionSphereModifier {
    fn apply(&self, init_layout: &mut InitLayout) {
        let radius_code = match self.dimension {
            ShapeDimension::Surface => {
                // Constant radius
                format!("let r = {};", self.radius.to_wgsl_string())
            }
            ShapeDimension::Volume => {
                // Radius uniformly distributed in [0:1], then scaled by ^(1/3) in 3D
                // to account for the increased surface covered by increased radii.
                // https://stackoverflow.com/questions/54544971/how-to-generate-uniform-random-points-inside-d-dimension-ball-sphere
                format!(
                    "var r = pow(rand(), 1./3.) * {};",
                    self.radius.to_wgsl_string()
                )
            }
        };

        let speed_code = match self.speed {
            SpeedVector::Normal(speed) | SpeedVector::Radial(speed) => {
                format!("ret.vel = dir * ({});", speed.to_wgsl_string())
            }
            SpeedVector::Local(speed_x, speed_y, speed_z) => {
                let rotation: Vec4 = self.rotation.into();
                format!(
                    "
    let rot = {};
    ret.vel = rotate_point(vec3<f32>({}, {}, {}), rot);
                        ",
                    rotation.to_wgsl_string(),
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
            SpeedVector::World(speed_x, speed_y, speed_z) => {
                format!(
                    "ret.vel = vec3<f32>({}, {}, {});",
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
        };

        init_layout.position_code = format!(
            r##"
    // >>> [PositionSphereModifier]
    // Sphere center
    let c = {};
    // Sphere radius
    {}
    // Spawn randomly along the sphere surface using Archimedes's theorem
    var theta = rand() * tau;
    var z = rand() * 2. - 1.;
    var phi = acos(z);
    var sinphi = sin(phi);
    var x = sinphi * cos(theta);
    var y = sinphi * sin(theta);
    var dir = vec3<f32>(x, y, z);
    ret.pos = c + r * dir;
    // Speed code
    {}
    // <<< [PositionSphereModifier]
"##,
            self.center.to_wgsl_string(),
            radius_code,
            speed_code,
        );
    }
}

/// An initialization modifier spawning particles on a cube.
#[derive(Default, Clone, Copy)]
pub struct PositionCubeModifier {
    /// The cube center, relative to the emitter position.
    pub center: Vec3,
    /// The box extents (half-sizes).
    pub extents: Vec3,
    /// The box rotation
    pub rotation: Quat,
    /// The speed of the particles on spawn.
    pub speed: SpeedVector,
    /// The shape dimension to spawn from.
    pub dimension: ShapeDimension,
}

impl InitModifier for PositionCubeModifier {
    fn apply(&self, init_layout: &mut InitLayout) {
        let rotation: Vec4 = self.rotation.into();
        let extents_code = match self.dimension {
            ShapeDimension::Surface => {
                // Constant extents: box faces
                format!("let extents = {};", self.extents.to_wgsl_string())
            }
            ShapeDimension::Volume => {
                // Random extents in box
                format!(
                    "var extents = (rand3() - vec3<f32>(0.5, 0.5, 0.5)) * {};",
                    self.extents.to_wgsl_string()
                )
            }
        };

        let speed_code = match self.speed {
            SpeedVector::Normal(speed) => {
                format!(
                    "
    var normal = vec3<f32>(0., 0., 0.);
    normal[ si[0] ] = locked_axis_sign;
    ret.vel = rotate_point(normal * {}, rot);
                    ",
                    speed.to_wgsl_string()
                )
            }
            SpeedVector::Radial(speed) => {
                // Velocity away from box center
                format!("ret.vel = ret.pos * {};", speed.to_wgsl_string())
            }
            SpeedVector::Local(speed_x, speed_y, speed_z) => {
                format!(
                    "ret.vel = rotate_point(vec3<f32>({}, {}, {}), rot);",
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
            SpeedVector::World(speed_x, speed_y, speed_z) => {
                format!(
                    "ret.vel = vec3<f32>({}, {}, {});",
                    speed_x.to_wgsl_string(),
                    speed_y.to_wgsl_string(),
                    speed_z.to_wgsl_string()
                )
            }
        };

        init_layout.position_code = format!(
            r##"
    // >>> [PositionCubeModifier]
    // Box center
    let c = {0};
    // Box rotation
    let rot = {1};
    // Box extents
    {2}
    // Spawn randomly along the (possibly randomized) box surface
    // Choose a face randomly and find a point on that face
    var s = rand_positive_int(6u);
    var si = vec3<u32>(s, s + 1u, s + 2u) % 3u;
    // si[0] is the selected face: lock the corresponding coordinate
    var locked_axis_sign = 0.;
    if (s > 2u) {{
        locked_axis_sign = 1.;
    }} else {{
        locked_axis_sign = -1.;
    }}
    ret.pos[ si[0] ] = locked_axis_sign * .5 * extents[ si[0] ];
    
    // 2 degrees of freedom for the remaining coordinates
    ret.pos[ si[1] ] = (rand() - .5) * extents[ si[1] ];
    ret.pos[ si[2] ] = (rand() - .5) * extents[ si[2] ];
    
    // Apply rotation
    ret.pos = rotate_point(ret.pos, rot);
    
    // Speed
    {3}
    
    // Put position in world space
    ret.pos = ret.pos + c;
    // <<< [PositionCubeModifier]
"##,
            self.center.to_wgsl_string(),
            rotation.to_wgsl_string(),
            extents_code,
            speed_code,
        );
    }
}

/// A modifier modulating each particle's color by sampling a texture.
#[derive(Default, Clone)]
pub struct ParticleTextureModifier {
    /// The texture image to modulate the particle color with.
    pub texture: Handle<Image>,
}

impl RenderModifier for ParticleTextureModifier {
    fn apply(&self, render_layout: &mut RenderLayout) {
        render_layout.particle_texture = Some(self.texture.clone());
    }
}

/// A modifier modulating each particle's color over its lifetime with a gradient curve.
#[derive(Default, Clone)]
pub struct ColorOverLifetimeModifier {
    /// The color gradient defining the particle color based on its lifetime.
    pub gradient: Gradient<Vec4>,
}

impl RenderModifier for ColorOverLifetimeModifier {
    fn apply(&self, render_layout: &mut RenderLayout) {
        render_layout.lifetime_color_gradient = Some(self.gradient.clone());
    }
}

/// A modifier modulating each particle's size over its lifetime with a gradient curve.
#[derive(Default, Clone)]
pub struct SizeOverLifetimeModifier {
    /// The size gradient defining the particle size based on its lifetime.
    pub gradient: Gradient<Vec2>,
}

impl RenderModifier for SizeOverLifetimeModifier {
    fn apply(&self, render_layout: &mut RenderLayout) {
        render_layout.size_color_gradient = Some(self.gradient.clone());
    }
}

/// A modifier to apply a constant acceleration to all particles each frame.
///
/// This is typically used to apply some kind of gravity.
#[derive(Default, Clone, Copy)]
pub struct AccelModifier {
    /// The constant acceleration to apply to all particles in the effect each frame.
    pub accel: Vec3,
}

impl UpdateModifier for AccelModifier {
    fn apply(&self, layout: &mut UpdateLayout) {
        layout.accel = self.accel;
    }
}

/// Parameters for the components making the force field.
#[derive(Clone, Copy)]
pub struct ForceFieldParam {
    /// Position of the source of the force field.
    pub position: Vec3,
    /// Maximum radius of the sphere of influence, outside of which
    /// the force field is null.
    pub max_radius: f32,
    /// Minimum radius of the sphere of influence, inside of which
    /// the force field is null, avoiding the singularity at the source position.
    pub min_radius: f32,
    /// The intensity of the force is proportional to mass. Note that the particles_update.wgsl shader will
    /// ignore all subsequent force field components after it encounters a component with a mass of zero.
    /// To change the force from an attracting one to a repulsive one, simply set the mass to a negative value.
    pub mass: f32,
    /// The force field is proportional to `1 / distance^force_exponent`.
    pub force_exponent: f32,
    /// If set to true, the particles that enter within the `min_radius` will conform to a sphere around the
    /// source position, appearing like a recharging effect.
    pub conform_to_sphere: bool,
}

impl Default for ForceFieldParam {
    fn default() -> Self {
        // defaults to no force field (a mass of 0)
        ForceFieldParam {
            position: Vec3::new(0., 0., 0.),
            min_radius: 0.1,
            max_radius: 0.0,
            mass: 0.,
            force_exponent: 0.0,
            conform_to_sphere: false,
        }
    }
}

/// A modifier to apply a force field to all particles each frame. The force field is made up of
/// point sources, also called 'components'. The maximum number of components is set with [`FFNUM`].
#[derive(Default, Clone, Copy)]
pub struct ForceFieldModifier {
    /// Array of force field components.
    pub force_field: [ForceFieldParam; FFNUM],
}

impl ForceFieldModifier {
    /// Instantiate a ForceFieldModifier.
    ///
    /// # Panics
    ///
    /// Panics if the number of sources exceeds [`FFNUM`].
    pub fn new<T>(point_attractors: T) -> Self
    where
        T: IntoIterator<Item = ForceFieldParam>,
    {
        let mut force_field = [ForceFieldParam::default(); FFNUM];

        for (i, p_attractor) in point_attractors.into_iter().enumerate() {
            if i > FFNUM {
                panic!("Too many point attractors");
            }
            force_field[i] = p_attractor;
        }

        Self { force_field }
    }

    /// Perhaps will be deleted in the future.
    // Delete me?
    pub fn add_or_replace(&mut self, point_attractor: ForceFieldParam, index: usize) {
        self.force_field[index] = point_attractor;
    }
}

impl UpdateModifier for ForceFieldModifier {
    fn apply(&self, layout: &mut UpdateLayout) {
        layout.force_field = self.force_field;
    }
}
