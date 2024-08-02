use std::fs;

use bevy::{
    prelude::*,
    reflect::{DynamicEnum, DynamicVariant, Enum, EnumInfo, ReflectFromPtr, ReflectMut, TypeInfo},
};

use crate::{PreferencesDir, PreferencesGroup, PreferencesKey};

pub fn load_preferences(world: &mut World) {
    println!("Loading preferences");

    let prefs_dir = world.get_resource::<PreferencesDir>().unwrap();
    let prefs_file = prefs_dir.0.join("prefs.toml");

    let table = if prefs_file.exists() && prefs_file.is_file() {
        let prefs_str = match fs::read_to_string(&prefs_file) {
            Ok(prefs_str) => prefs_str,
            Err(e) => {
                error!("Error reading preferences file: {}", e);
                return;
            }
        };
        let table_value = match toml::from_str::<toml::Value>(&prefs_str) {
            Ok(table_value) => table_value,
            Err(e) => {
                error!("Error parsing preferences file: {}", e);
                return;
            }
        };

        match table_value {
            toml::Value::Table(table) => table,
            _ => {
                error!("Preferences file must be a table");
                return;
            }
        }
    } else {
        return;
    };

    let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
    let registry_read = registry.read();
    let resources = world
        .iter_resources()
        .map(|(res, _)| (res.type_id(), res.id()))
        .collect::<Vec<_>>();
    for (res_type_id, res_id) in resources {
        if let Some(tid) = res_type_id {
            if let Some(treg) = registry_read.get(tid) {
                let type_name = treg.type_info().type_path();
                match treg.type_info() {
                    bevy::reflect::TypeInfo::Struct(stty) => {
                        if let Some(_group) = stty.custom_attributes().get::<PreferencesGroup>() {
                            // let group = table.get(group.0).unwrap().as_table().unwrap();
                            warn!("Preferences: Structs not supported yet: {}", type_name);
                        } else if let Some(_key) = stty.custom_attributes().get::<PreferencesKey>()
                        {
                            warn!("Preferences: Structs not supported yet: {}", type_name);
                        }
                    }
                    bevy::reflect::TypeInfo::TupleStruct(tsty) => {
                        let group_attr = tsty.custom_attributes().get::<PreferencesGroup>();
                        let key_attr = tsty.custom_attributes().get::<PreferencesKey>();
                        let mut ptr = world.get_resource_mut_by_id(res_id).unwrap();
                        let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                        let ReflectMut::TupleStruct(tuple_struct) =
                            unsafe { reflect_from_ptr.as_reflect_mut(ptr.as_mut()) }.reflect_mut()
                        else {
                            panic!("Expected TupleStruct");
                        };
                        if group_attr.is_some() || key_attr.is_some() {
                            maybe_load_tuple_struct(tuple_struct, group_attr, key_attr, &table);
                        } else if tsty
                            .type_path()
                            .starts_with("bevy_state::state::resources::State<")
                        {
                            let state_reflect = tuple_struct.field_mut(0).unwrap();
                            let state_info = state_reflect.get_represented_type_info().unwrap();
                            let field_reflect_mut = state_reflect.reflect_mut();
                            match (state_info, field_reflect_mut) {
                                (TypeInfo::Struct(_), ReflectMut::Struct(_)) => todo!(),
                                (TypeInfo::TupleStruct(_), ReflectMut::TupleStruct(_)) => {
                                    todo!()
                                }
                                (TypeInfo::Enum(enum_ty), ReflectMut::Enum(enum_mut)) => {
                                    maybe_load_enum(enum_ty, enum_mut, &table);
                                }
                                _ => {}
                            }
                        }
                    }
                    bevy::reflect::TypeInfo::Enum(ety) => {
                        if let Some(group) = ety.custom_attributes().get::<PreferencesGroup>() {
                            let _group = table.get(group.0).unwrap().as_table().unwrap();
                            warn!("Preferences: Enums not supported yet: {}", type_name);
                        } else if let Some(_key) = ety.custom_attributes().get::<PreferencesKey>() {
                            warn!("Preferences: Enums not supported yet: {}", type_name);
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
}

fn maybe_load_tuple_struct(
    tuple_struct: &mut dyn TupleStruct,
    group_attr: Option<&PreferencesGroup>,
    key_attr: Option<&PreferencesKey>,
    table: &toml::Table,
) {
    if let Some(group) = group_attr {
        let Some(group_value) = table.get(group.0) else {
            return;
        };
        let Some(group) = group_value.as_table() else {
            return;
        };

        if let Some(key) = key_attr {
            load_tuple_struct(tuple_struct, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        load_tuple_struct(tuple_struct, key.0, table);
    }
}

fn load_tuple_struct(tuple_struct: &mut dyn TupleStruct, key: &'static str, table: &toml::Table) {
    if tuple_struct.field_len() == 1 {
        let field_mut = tuple_struct.field_mut(0).unwrap();
        match field_mut.get_represented_type_info().unwrap() {
            TypeInfo::Struct(_) => todo!(),
            TypeInfo::TupleStruct(_) => todo!(),
            TypeInfo::Tuple(_) => todo!(),
            TypeInfo::List(_) => todo!(),
            TypeInfo::Array(_) => todo!(),
            TypeInfo::Map(_) => todo!(),
            TypeInfo::Enum(_) => todo!(),
            TypeInfo::Value(val) => match table.get(key) {
                Some(toml::Value::Float(float_val)) => {
                    if let Some(float_field) = field_mut.downcast_mut::<f32>() {
                        float_field.apply((*float_val as f32).as_reflect());
                    } else {
                        warn!("Preferences: Unsupported type: {:?}", val);
                    }
                }
                _ => {
                    warn!("Preferences: unsupported type: {}", key);
                }
            },
        }
    }
}

fn maybe_load_enum(enum_ty: &EnumInfo, enum_mut: &mut dyn Enum, table: &toml::Table) {
    let group_attr = enum_ty.custom_attributes().get::<PreferencesGroup>();
    let key_attr = enum_ty.custom_attributes().get::<PreferencesKey>();
    if let Some(group) = group_attr {
        let Some(group_value) = table.get(group.0) else {
            return;
        };
        let Some(group) = group_value.as_table() else {
            return;
        };

        if let Some(key) = key_attr {
            load_enum(enum_ty, enum_mut, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        load_enum(enum_ty, enum_mut, key.0, table);
    }
}

fn load_enum(enum_ty: &EnumInfo, enum_mut: &mut dyn Enum, key: &'static str, table: &toml::Table) {
    match table.get(key) {
        Some(toml::Value::String(s)) => {
            let Some(variant) = enum_ty.variant(s) else {
                warn!("Preferences: Unknown variant: {}", s);
                return;
            };
            if variant.name() != enum_mut.variant_name() {
                let dynamic_enum = DynamicEnum::new(variant.name(), DynamicVariant::Unit);
                enum_mut.apply(dynamic_enum.as_reflect());
            }
        }
        None => {}
        _ => {
            warn!("Preferences: unsupported type: {}", key);
        }
    };
}
