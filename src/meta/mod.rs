use std::{marker::PhantomData, sync::Arc};

use bevy::{prelude::*, utils::HashMap};

/// Common fields for game metadata.
#[derive(Debug, Component)]
pub struct MetaHeader {
    /// Lookup key
    pub name: String,

    /// Catalog this was loaded from
    pub catalog: Arc<String>,
}

/// Human-readable display name for metadata.
pub struct DisplayName(String);

/// Alias names for metadata.
pub struct MetadataAlias(Vec<String>);

/// Handle to a metadata item.
pub struct MetaRef {
    handle: Handle<MetadataCatalog>,
    key: String,
}

impl Drop for MetaRef {
    fn drop(&mut self) {
        println!("Dropping metadata ref: {:?}", self.key);
    }
}

/// Handle to a metadata item.
#[derive(Clone)]
pub struct MetaHandle<T> {
    mref: Arc<MetaRef>,
    marker: PhantomData<T>,
}

/// Trait that defines loadable game data.
pub trait Metadata {
    fn name(&self) -> &str;
    fn display_name(&self) -> Option<&str>;
    fn alias(&self) -> &[String];
    fn catalog(&self) -> &String;
}

/// An asset that stores multiple metadata items.
#[derive(Asset)]
pub struct MetadataCatalog {
    entries: HashMap<String, Box<dyn Metadata>>,
}

impl MetadataCatalog {
    fn get<T>(&self, name: &str) -> MetaHandle<T> {
        match self.entries.get(name) {
            Some(entry) => Some(entry.as_ref().as_any().downcast_ref::<T>().unwrap()),
            None => None,
        }
    }
}

/// Global cache of metadata catalogs.
#[derive(Debug, Resource)]
pub struct MetadataCache {
    cache: lru::LruCache<String, Arc<MetadataCatalog<'static>>>,
}

impl MetadataCache {
    fn get<T>(&self, name: &str) -> MetaHandle<T> {
        match self.entries.get(name) {
            Some(entry) => Some(entry.as_ref().as_any().downcast_ref::<T>().unwrap()),
            None => None,
        }
    }
}

impl MetadataCache {
    // fn get<T>(&self, path: &AssetPath) -> Option<&T> {
    //     todo!()
    // }
}

// Examples

// trait WallProto: Metadata {
//     fn as_wall(&self) -> &dyn WallProto;
// }

// impl WallProto for dyn Metadata {
//     fn as_wall(&self) -> &dyn WallProto {
//         self
//     }
// }
