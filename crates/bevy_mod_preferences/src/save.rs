use std::fs;

use bevy::{
    ecs::world::Command,
    prelude::*,
    reflect::{Enum, EnumInfo, ReflectFromPtr, ReflectRef, TypeInfo, VariantType},
};

use crate::{PreferencesChanged, PreferencesDir, PreferencesGroup, PreferencesKey};

#[derive(Default, PartialEq)]
pub enum SavePreferences {
    /// Save prefernces only if they have changed (based on [`PreferencesChanged` resource]).
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferences {
    fn apply(self, world: &mut World) {
        let mut changed = world.get_resource_mut::<PreferencesChanged>().unwrap();
        if changed.0 || self == SavePreferences::Always {
            changed.0 = false;
            let prefs_dir = world.get_resource::<PreferencesDir>().unwrap();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            let registry_read = registry.read();
            let prefs_file_new = prefs_dir.0.join("prefs.toml.new");
            let prefs_file = prefs_dir.0.join("prefs.toml");
            let mut table = toml::Table::new();
            for (res, _) in world.iter_resources() {
                if let Some(tid) = res.type_id() {
                    if let Some(treg) = registry_read.get(tid) {
                        match treg.type_info() {
                            bevy::reflect::TypeInfo::Struct(stty) => {
                                if let Some(_group) =
                                    stty.custom_attributes().get::<PreferencesGroup>()
                                {
                                    warn!("Preferences: Structs not supported yet: {}", res.name());
                                } else if let Some(_key) =
                                    stty.custom_attributes().get::<PreferencesKey>()
                                {
                                    warn!("Preferences: Structs not supported yet: {}", res.name());
                                }
                            }
                            bevy::reflect::TypeInfo::TupleStruct(tsty) => {
                                let group_attr = tsty.custom_attributes().get::<PreferencesGroup>();
                                let key_attr = tsty.custom_attributes().get::<PreferencesKey>();
                                let ptr = world.get_resource_by_id(res.id()).unwrap();
                                let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                                let ReflectRef::TupleStruct(tuple_struct) =
                                    unsafe { reflect_from_ptr.as_reflect(ptr) }.reflect_ref()
                                else {
                                    panic!("Expected TupleStruct");
                                };
                                if group_attr.is_some() || key_attr.is_some() {
                                    maybe_save_tuple_struct(
                                        tuple_struct,
                                        group_attr,
                                        key_attr,
                                        &mut table,
                                    );
                                } else if tsty
                                    .type_path()
                                    .starts_with("bevy_state::state::resources::State<")
                                {
                                    let state_reflect = tuple_struct.field(0).unwrap();
                                    let state_info =
                                        state_reflect.get_represented_type_info().unwrap();
                                    let field_reflect_ref = state_reflect.reflect_ref();
                                    match (state_info, field_reflect_ref) {
                                        (TypeInfo::Struct(_), ReflectRef::Struct(_)) => todo!(),
                                        (TypeInfo::TupleStruct(_), ReflectRef::TupleStruct(_)) => {
                                            todo!()
                                        }
                                        (TypeInfo::Enum(enum_ty), ReflectRef::Enum(enum_ref)) => {
                                            maybe_save_enum(enum_ty, enum_ref, &mut table);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            bevy::reflect::TypeInfo::Enum(ety) => {
                                if let Some(_group) =
                                    ety.custom_attributes().get::<PreferencesGroup>()
                                {
                                    warn!("Preferences: Enums not supported yet: {}", res.name());
                                } else if let Some(_key) =
                                    ety.custom_attributes().get::<PreferencesKey>()
                                {
                                    warn!("Preferences: Enums not supported yet: {}", res.name());
                                }
                                // warn!("Preferences: Enums not supported yet: {}", res.name());
                            }

                            // Other types cannot be preferences since they don't have attributes.
                            _ => {}
                        }
                    }
                    // println!("Saving preferences for {:?}", res.name());
                }
            }

            // Recursively create the preferences directory if it doesn't exist.
            let mut dir_builder = std::fs::DirBuilder::new();
            dir_builder.recursive(true);
            if let Err(e) = dir_builder.create(prefs_dir.0.clone()) {
                warn!("Could not create preferences directory: {:?}", e);
                return;
            }

            // Write to temporary file.
            if let Err(e) = fs::write(&prefs_file_new, table.to_string()) {
                warn!("Could not write preferences file: {:?}", e);
                return;
            }

            // Replace old prefs file with new one.
            if let Err(e) = fs::rename(&prefs_file_new, &prefs_file) {
                warn!("Could not save preferences file: {:?}", e);
            }

            // info!("Saved: {:?}", prefs_file);
            // println!("Preferences:\n{}\n", table);
        }
    }
}

fn maybe_save_tuple_struct(
    tuple_struct: &dyn TupleStruct,
    group_attr: Option<&PreferencesGroup>,
    key_attr: Option<&PreferencesKey>,
    table: &mut toml::Table,
) {
    if let Some(group) = group_attr {
        let group = table
            .entry(group.0.to_string())
            .or_insert(toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        if let Some(key) = key_attr {
            save_tuple_struct(tuple_struct, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        save_tuple_struct(tuple_struct, key.0, table);
    }
}

fn save_tuple_struct(tuple_struct: &dyn TupleStruct, key: &'static str, table: &mut toml::Table) {
    if tuple_struct.field_len() == 1 {
        let field_reflect = tuple_struct.field(0).unwrap();
        match field_reflect.get_represented_type_info().unwrap() {
            TypeInfo::Struct(_) => todo!(),
            TypeInfo::TupleStruct(_) => todo!(),
            TypeInfo::Tuple(_) => todo!(),
            TypeInfo::List(_) => todo!(),
            TypeInfo::Array(_) => todo!(),
            TypeInfo::Map(_) => todo!(),
            TypeInfo::Enum(_) => todo!(),
            TypeInfo::Value(val) => {
                if let Some(f) = field_reflect.downcast_ref::<f32>() {
                    let v = toml::Value::Float(*f as f64);
                    table.insert(key.to_string(), v);
                } else {
                    warn!("Preferences: Unsupported type: {:?}", val);
                }
            }
        }
    }
}

fn maybe_save_enum(enum_ty: &EnumInfo, enum_ref: &dyn Enum, table: &mut toml::Table) {
    let group_attr = enum_ty.custom_attributes().get::<PreferencesGroup>();
    let key_attr = enum_ty.custom_attributes().get::<PreferencesKey>();
    if let Some(group) = group_attr {
        let group = table
            .entry(group.0.to_string())
            .or_insert(toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        if let Some(key) = key_attr {
            save_enum(enum_ref, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        save_enum(enum_ref, key.0, table);
    }
}

fn save_enum(enum_ref: &dyn Enum, key: &'static str, table: &mut toml::Table) {
    if enum_ref.variant_type() != VariantType::Unit {
        todo!("Figure out how to encode non-unit enums in TOML");
    }
    let v = toml::Value::String(enum_ref.variant_name().to_string());
    table.insert(key.to_string(), v);
}
