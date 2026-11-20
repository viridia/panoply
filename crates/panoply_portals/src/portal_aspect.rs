use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::*;
use panoply_scenery::{FIXTURE_TYPE, WALL_TYPE};
use serde::{Deserialize, Serialize};

/// The side of a portal's aperture it can be seen and crossed from.
#[derive(Debug, Reflect, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortalSide {
    #[default]
    Both,
    Front,
    Back,
}

/// The aperture of a portal, on a wall or fixture exemplar.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct Portal {
    /// The aperture's width and height, centered on `offset`.
    #[reflect(@DisplayName("Size"))]
    pub size: Vec2,

    /// Rotation of the aperture.
    #[reflect(@DisplayName("X rotation"))]
    pub x_rotation: Option<f32>,
    #[reflect(@DisplayName("Y rotation"))]
    pub y_rotation: Option<f32>,
    #[reflect(@DisplayName("Z rotation"))]
    pub z_rotation: Option<f32>,

    /// Offset of the aperture from the center of the entity.
    #[reflect(@DisplayName("Offset"))]
    pub offset: Vec3,

    /// The side the aperture is active from.
    #[reflect(@DisplayName("Side"))]
    pub side: PortalSide,

    /// How far an entity crossing the portal is displaced, so that it is not left standing in it.
    #[reflect(@DisplayName("Displacement"))]
    pub displacement: Option<f32>,
}

impl Aspect for Portal {
    fn name(&self) -> &str {
        "Portal"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Portal> = RemoveComponent::<Portal>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

/// The far end of a portal, per instance: its realm and place.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Aspect, Default)]
pub struct PortalTarget {
    #[reflect(@DisplayName("Realm"))]
    pub realm: String,
    #[reflect(@DisplayName("Position"))]
    pub pos: Vec3,
}

impl Aspect for PortalTarget {
    fn name(&self) -> &str {
        "PortalTarget"
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<PortalTarget> = RemoveComponent::<PortalTarget>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::reflect::{
        serde::{TypedReflectDeserializer, TypedReflectSerializer},
        FromReflect, TypeRegistry,
    };
    use serde::de::DeserializeSeed;

    /// A catalog's `Portal`, which leaves out the fields at `None`, reads through reflection as the
    /// loader reads it, and writes back the same apart from those fields, written as `null`.
    #[test]
    fn portal_reads_and_writes_as_a_catalog_has_it() {
        let mut registry = TypeRegistry::default();
        registry.register::<Portal>();
        registry.register::<PortalSide>();
        let text = r#"{"side":"front","size":[2.0,1.6],"offset":[0.3,0.8,0.5]}"#;
        let registration = registry.get(std::any::TypeId::of::<Portal>()).unwrap();
        let reflected = TypedReflectDeserializer::new(registration, &registry)
            .deserialize(&mut serde_json::Deserializer::from_str(text))
            .unwrap();
        let portal = Portal::from_reflect(reflected.as_ref()).unwrap();
        assert_eq!(portal.side, PortalSide::Front);
        assert_eq!(portal.size, Vec2::new(2.0, 1.6));
        assert_eq!(portal.x_rotation, None);

        let written: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&TypedReflectSerializer::new(&portal, &registry)).unwrap(),
        )
        .unwrap();
        let mut written = written.as_object().unwrap().clone();
        written.retain(|_, value| !value.is_null());
        let read: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(serde_json::Value::Object(written), read);
    }

    #[test]
    fn test_serialize_json() {
        let target = PortalTarget {
            realm: "Realm".to_string(),
            pos: Vec3::new(1.0, 2.0, 3.0),
        };
        let ser = serde_json::to_string(&target).unwrap();
        assert_eq!(ser, r#"{"realm":"Realm","pos":[1.0,2.0,3.0]}"#);
        let des = serde_json::from_str::<PortalTarget>(&ser).unwrap();
        assert_eq!(des.realm, "Realm");
        assert_eq!(des.pos, Vec3::new(1.0, 2.0, 3.0));
    }
}
