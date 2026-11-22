//! The portal camera's projection: the primary camera's perspective, cut to the aperture's
//! rectangle on screen and clipped at the far aperture's plane (D21).
//!
//! Bevy's [`SubCameraView`] makes the same cut for rendering, but not for culling: a camera's
//! frustum comes from `get_clip_from_view`, which ignores the sub view, and the camera system
//! resets the aspect ratio to the camera's own viewport. So the cut is this projection's matrix
//! itself, and its frustum and shadow cascades fit the aperture.

use bevy::{
    camera::{CameraProjection, SubCameraView},
    math::{vec4, Vec3A},
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct PortalProjection {
    /// The primary camera's vertical field of view, in radians.
    pub fov: f32,
    /// The primary camera's aspect ratio, width over height.
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    /// The part of the primary camera's view this camera renders, in its normalized device
    /// coordinates (-1 to 1, y up).
    pub rect: Rect,
    /// The far aperture's plane in view space, as `PerspectiveProjection::near_clip_plane` takes
    /// one: a unit normal pointing away from the camera, and w the negative distance to it.
    /// Everything nearer than the plane is clipped. `None` keeps the near plane.
    pub clip_plane: Option<Vec4>,
}

impl Default for PortalProjection {
    fn default() -> Self {
        let perspective = PerspectiveProjection::default();
        Self {
            fov: perspective.fov,
            aspect_ratio: perspective.aspect_ratio,
            near: perspective.near,
            far: perspective.far,
            rect: Rect::new(-1., -1., 1., 1.),
            clip_plane: None,
        }
    }
}

impl CameraProjection for PortalProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        // The infinite reversed-Z perspective, with x and y mapped so that `rect` fills clip space.
        let focal = 1. / (0.5 * self.fov).tan();
        let size = self.rect.size();
        let center = self.rect.min + self.rect.max;
        let mut matrix = Mat4::from_cols(
            vec4(2. / size.x * focal / self.aspect_ratio, 0., 0., 0.),
            vec4(0., 2. / size.y * focal, 0., 0.),
            vec4(center.x / size.x, center.y / size.y, 0., -1.),
            vec4(0., 0., self.near, 0.),
        );
        if let Some(plane) = self.clip_plane {
            clip_at_plane(&mut matrix, plane);
        }
        matrix
    }

    fn get_clip_from_view_for_sub(&self, _sub_view: &SubCameraView) -> Mat4 {
        self.get_clip_from_view()
    }

    /// The aspect ratio is the primary camera's, not this camera's viewport's.
    fn update(&mut self, _width: f32, _height: f32) {}

    fn far(&self) -> f32 {
        self.far
    }

    /// Ordered as `CameraProjection` asks: bottom right, top right, top left, bottom left, near
    /// before far. The depths are view-space z, so negative in front of the camera.
    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        let tan_half_fov = (0.5 * self.fov).tan();
        let corners = |z: f32| {
            let y = z.abs() * tan_half_fov;
            let x = y * self.aspect_ratio;
            let (left, right) = (self.rect.min.x * x, self.rect.max.x * x);
            let (bottom, top) = (self.rect.min.y * y, self.rect.max.y * y);
            [
                Vec3A::new(right, bottom, z),
                Vec3A::new(right, top, z),
                Vec3A::new(left, top, z),
                Vec3A::new(left, bottom, z),
            ]
        };
        let [a, b, c, d] = corners(z_near);
        let [e, f, g, h] = corners(z_far);
        [a, b, c, d, e, f, g, h]
    }
}

/// Moves the near plane of a reversed-Z perspective `matrix` onto `plane`, after Lengyel's
/// oblique near-plane clipping, as Bevy's `PerspectiveProjection` does for its own matrix (in its
/// private `adjust_perspective_matrix_for_clip_plane`), whose derivation this follows. The corner
/// of clip space opposite the plane is taken from the plane in clip space, since the matrix here
/// is off-axis, where the signs of the view-space plane may differ.
fn clip_at_plane(matrix: &mut Mat4, plane: Vec4) {
    let clip_plane = matrix.inverse().transpose() * plane;
    let corner = matrix.inverse() * vec4(clip_plane.x.signum(), clip_plane.y.signum(), 0., 1.);
    let scale = -corner.z / plane.dot(corner);
    let row = Vec4::NEG_Z - plane * scale;
    matrix.x_axis.z = row.x;
    matrix.y_axis.z = row.y;
    matrix.z_axis.z = row.z;
    matrix.w_axis.z = row.w;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> PortalProjection {
        PortalProjection {
            fov: 0.69,
            aspect_ratio: 16. / 9.,
            near: 0.5,
            far: 100.,
            ..default()
        }
    }

    /// Clip-space depth over w of a view-space point: in [0, 1] when drawn, above 1 when nearer
    /// than the near plane.
    fn depth(matrix: Mat4, point: Vec3) -> f32 {
        let clip = matrix * point.extend(1.);
        clip.z / clip.w
    }

    /// Cut to the whole view, the projection is the primary camera's own.
    #[test]
    fn whole_view_is_the_perspective() {
        let portal = projection();
        let perspective = PerspectiveProjection {
            fov: portal.fov,
            aspect_ratio: portal.aspect_ratio,
            near: portal.near,
            far: portal.far,
            near_clip_plane: vec4(0., 0., -1., -portal.near),
        };
        assert!(portal
            .get_clip_from_view()
            .abs_diff_eq(perspective.get_clip_from_view(), 1e-6));
    }

    /// A point keeps its place on the primary camera's screen: where the cut puts it, mapped back
    /// through the rect, is where the whole view puts it.
    #[test]
    fn cut_keeps_screen_positions() {
        let whole = projection();
        let cut = PortalProjection {
            rect: Rect::new(0.2, -0.6, 0.5, -0.1),
            ..whole.clone()
        };
        let point = Vec3::new(1.3, -2.4, -9.);
        let whole_clip = whole.get_clip_from_view() * point.extend(1.);
        let cut_clip = cut.get_clip_from_view() * point.extend(1.);
        let whole_ndc = whole_clip.xy() / whole_clip.w;
        let cut_ndc = cut_clip.xy() / cut_clip.w;
        let back = cut.rect.min + (cut_ndc + 1.) / 2. * cut.rect.size();
        assert!(back.abs_diff_eq(whole_ndc, 1e-5), "{back} {whole_ndc}");
    }

    /// With a clip plane, what lies between the camera and the plane is clipped, and what lies
    /// beyond it is drawn, off-axis as well as on.
    #[test]
    fn clip_plane_clips_what_is_nearer() {
        // A plane 6 m ahead, tilted, as a far aperture seen from above at an angle is.
        let normal = Vec3::new(0.3, -0.5, -1.).normalize();
        let on_plane = Vec3::new(0., -1., -6.);
        let plane = normal.extend(-normal.dot(on_plane));
        for rect in [Rect::new(-1., -1., 1., 1.), Rect::new(0.2, -0.6, 0.5, -0.1)] {
            let portal = PortalProjection {
                rect,
                clip_plane: Some(plane),
                ..projection()
            };
            let matrix = portal.get_clip_from_view();
            let nearer = on_plane - normal * 1.;
            let beyond = on_plane + normal * 1.;
            let far_beyond = on_plane + normal * 40.;
            assert!(depth(matrix, nearer) > 1., "nearer: {}", depth(matrix, nearer));
            for point in [beyond, far_beyond] {
                let d = depth(matrix, point);
                assert!((0. ..=1.).contains(&d), "beyond: {d}");
            }
        }
    }

    /// The frustum's corners lie on the cut: projected, they land on the rect's corners.
    #[test]
    fn corners_lie_on_the_rect() {
        let portal = PortalProjection {
            rect: Rect::new(0.2, -0.6, 0.5, -0.1),
            ..projection()
        };
        let whole = projection().get_clip_from_view();
        let corners = portal.get_frustum_corners(-1., -20.);
        let expected = [
            Vec2::new(0.5, -0.6),
            Vec2::new(0.5, -0.1),
            Vec2::new(0.2, -0.1),
            Vec2::new(0.2, -0.6),
        ];
        for (i, corner) in corners.iter().enumerate() {
            let clip = whole * Vec3::from(*corner).extend(1.);
            let ndc = clip.xy() / clip.w;
            assert!(ndc.abs_diff_eq(expected[i % 4], 1e-5), "{i}: {ndc}");
        }
    }
}
