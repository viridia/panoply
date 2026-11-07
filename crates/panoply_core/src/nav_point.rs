use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

/// A named place to travel to. Fields serialize in the file's order, and absent ones stay absent.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NavPoint {
    /// Unique name of this nav point, such as `"stick-village"`.
    pub name: String,

    /// Name of the realm holding this nav point.
    pub realm: String,

    /// Where a traveler arrives, in meters; saved to the centimetre (`serialize_centimetres`).
    #[serde(serialize_with = "serialize_centimetres")]
    pub pos: Vec3,

    /// Human-readable name, if it differs from `name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,

    /// The icon marking this nav point on the in-game map, which shows only those that have one:
    /// `village`, `town`, `keep`, `cave`, `ruin` or `camp`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Writes a position rounded to the centimetre: finer is noise, such as the `4.7e-15` heights
/// faery's math left on ground at 0, and arrival needs no more, since a traveler is set on the
/// ground below the point. `+ 0.0` turns a rounded `-0.0` into `0.0`.
fn serialize_centimetres<S: Serializer>(pos: &Vec3, serializer: S) -> Result<S::Ok, S::Error> {
    let round = |v: f32| (v * 100.0).round() / 100.0 + 0.0;
    [round(pos.x), round(pos.y), round(pos.z)].serialize(serializer)
}

impl NavPoint {
    pub fn display_name(&self) -> &str {
        self.caption.as_deref().unwrap_or(&self.name)
    }
}

/// Every nav point in the world, from `world.locations.json`.
#[derive(TypePath, Asset)]
pub struct NavPointsAsset(pub Vec<NavPoint>);

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum NavPointsLoaderError {
    #[error("Could not load nav points: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode nav points: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Default, TypePath)]
pub struct NavPointsLoader;

impl AssetLoader for NavPointsLoader {
    type Asset = NavPointsAsset;
    type Error = NavPointsLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(NavPointsAsset(serde_json::from_slice(&bytes)?))
    }

    fn extensions(&self) -> &[&str] {
        &["locations.json"]
    }
}

/// Keeps the nav points loaded, and finds them in `Assets<NavPointsAsset>`.
#[derive(Resource)]
pub struct NavPointsHandleResource(pub Handle<NavPointsAsset>);

impl FromWorld for NavPointsHandleResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        NavPointsHandleResource(server.load("world.locations.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The authored file decodes, and each point names a realm that exists, since `GotoLocation`
    /// to an unknown realm does nothing.
    #[test]
    fn authored_nav_points_name_real_realms() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let bytes = std::fs::read(assets.join("world.locations.json")).unwrap();
        let points: Vec<NavPoint> = serde_json::from_slice(&bytes).unwrap();
        assert!(!points.is_empty());
        for point in &points {
            let realm = assets.join(format!("realms/{}.realm.json", point.realm));
            assert!(realm.exists(), "{} is in {}", point.name, point.realm);
        }
    }

    /// Serializing the authored points and reading them back gives the same points, positions
    /// to within the half centimetre that rounding moves them.
    #[test]
    fn authored_nav_points_round_trip() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let bytes = std::fs::read(assets.join("world.locations.json")).unwrap();
        let points: Vec<NavPoint> = serde_json::from_slice(&bytes).unwrap();
        let written = serde_json::to_vec(&points).unwrap();
        let read: Vec<NavPoint> = serde_json::from_slice(&written).unwrap();
        assert_eq!(read.len(), points.len());
        for (read, point) in read.iter().zip(&points) {
            assert!(read.pos.abs_diff_eq(point.pos, 0.005), "{} moved", point.name);
            assert_eq!(NavPoint { pos: point.pos, ..read.clone() }, *point);
        }
    }

    #[test]
    fn positions_save_to_the_centimetre() {
        let point = NavPoint {
            name: "warehouse".into(),
            realm: "playground".into(),
            pos: Vec3::new(41.59800604609949, 4.733259878704557e-15, -1e-9),
            caption: None,
            icon: None,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains(r#""pos":[41.6,0.0,0.0]"#), "{json}");
    }

    #[test]
    fn display_name_prefers_the_caption() {
        let mut point = NavPoint {
            name: "stick-village".into(),
            realm: "playground".into(),
            pos: Vec3::ZERO,
            caption: None,
            icon: None,
        };
        assert_eq!(point.display_name(), "stick-village");
        point.caption = Some("Stick Village".into());
        assert_eq!(point.display_name(), "Stick Village");
    }
}
