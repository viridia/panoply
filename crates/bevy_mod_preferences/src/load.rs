use std::fs;
use thiserror::Error;

use bevy::{
    prelude::*,
    reflect::{
        DynamicEnum, DynamicTuple, DynamicVariant, Enum, EnumInfo, ReflectFromPtr, ReflectMut,
        TypeInfo, VariantInfo,
    },
};

use crate::{PreferencesDir, PreferencesGroup, PreferencesKey};

pub fn load_preferences(world: &mut World) {
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
    let resources = world
        .iter_resources()
        .map(|(res, _)| (res.type_id(), res.id()))
        .collect::<Vec<_>>();
    for (res_type_id, res_id) in resources {
        if let Some(tid) = res_type_id {
            if let Some(treg) = registry.read().get(tid) {
                let type_name = treg.type_info().type_path();
                match treg.type_info() {
                    TypeInfo::Struct(stty) => {
                        let group_attr = stty.custom_attributes().get::<PreferencesGroup>();
                        let key_attr = stty.custom_attributes().get::<PreferencesKey>();
                        if group_attr.is_some() || key_attr.is_some() {
                            let mut ptr = world.get_resource_mut_by_id(res_id).unwrap();
                            let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                            let ReflectMut::Struct(strct) =
                                unsafe { reflect_from_ptr.as_reflect_mut(ptr.as_mut()) }
                                    .reflect_mut()
                            else {
                                panic!("Expected Struct");
                            };
                            maybe_load_struct(&registry, strct, group_attr, key_attr, &table);
                        }
                    }

                    TypeInfo::TupleStruct(tsty) => {
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
                            maybe_load_tuple_struct(
                                &registry,
                                tuple_struct,
                                group_attr,
                                key_attr,
                                &table,
                            );
                        } else if tsty
                            .type_path()
                            .starts_with("bevy_state::state::resources::NextState<")
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

                    TypeInfo::Enum(ety) => {
                        if let Some(group) = ety.custom_attributes().get::<PreferencesGroup>() {
                            let _group = table.get(group.0).unwrap().as_table().unwrap();
                            warn!("Preferences: Enums not supported yet: {}", type_name);
                        } else if let Some(_key) = ety.custom_attributes().get::<PreferencesKey>() {
                            warn!("Preferences: Enums not supported yet: {}", type_name);
                        } else if ety
                            .type_path()
                            .starts_with("bevy_state::state::resources::NextState<")
                        {
                            let Some(VariantInfo::Tuple(pending_ty)) = ety.variant("Pending")
                            else {
                                panic!("Expected Pending variant");
                            };
                            let state_ty = pending_ty.field_at(0).unwrap();
                            let state_type_id = state_ty.type_id();
                            let rr = registry.read();
                            let Some(state_type_reg) = rr.get(state_type_id) else {
                                warn!(
                                    "Expected state type registration for {}",
                                    state_ty.type_path()
                                );
                                continue;
                            };
                            let state_info = state_type_reg.type_info();
                            let group_attr = match state_info {
                                TypeInfo::Struct(_) => todo!(),
                                TypeInfo::TupleStruct(_) => todo!(),
                                TypeInfo::Enum(enum_ty) => {
                                    enum_ty.custom_attributes().get::<PreferencesGroup>()
                                }
                                _ => None,
                            };
                            let key_attr = match state_info {
                                TypeInfo::Struct(_) => todo!(),
                                TypeInfo::TupleStruct(_) => todo!(),
                                TypeInfo::Enum(enum_ty) => {
                                    enum_ty.custom_attributes().get::<PreferencesKey>()
                                }
                                _ => None,
                            };
                            if group_attr.is_none() && key_attr.is_none() {
                                continue;
                            }

                            let Some(reflect_default) = state_type_reg.data::<ReflectDefault>()
                            else {
                                warn!("No ReflectDefault for {}", state_ty.type_path());
                                continue;
                            };
                            let mut default_state = reflect_default.default();
                            let change = match (state_info, default_state.reflect_mut()) {
                                (TypeInfo::Struct(_), ReflectMut::Struct(_)) => false,
                                (TypeInfo::TupleStruct(_), ReflectMut::TupleStruct(_)) => false,
                                (TypeInfo::Enum(enum_ty), ReflectMut::Enum(enum_mut)) => {
                                    maybe_load_enum(enum_ty, enum_mut, &table)
                                }
                                _ => false,
                            };

                            if change {
                                let mut ptr = world.get_resource_mut_by_id(res_id).unwrap();
                                let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                                let ReflectMut::Enum(enum_mut) =
                                    unsafe { reflect_from_ptr.as_reflect_mut(ptr.as_mut()) }
                                        .reflect_mut()
                                else {
                                    panic!("Expected Enum");
                                };

                                let mut tuple = DynamicTuple::default();
                                tuple.insert_boxed(default_state);
                                let dynamic_enum =
                                    DynamicEnum::new("Pending", DynamicVariant::Tuple(tuple));
                                enum_mut.apply(dynamic_enum.as_reflect());
                            }
                        }
                    }

                    // Other types cannot be preferences since they don't have attributes.
                    _ => {}
                }
            }
            // println!("Saving preferences for {:?}", res.name());
        }
    }
}

fn maybe_load_struct(
    registry: &AppTypeRegistry,
    strct: &mut dyn Struct,
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

        if let Some(_key) = key_attr {
            todo!();
        } else {
            // TODO: Need to derive key name from tuple struct name
            load_struct(registry, strct, group);
            // todo!();
        }
    } else if let Some(_key) = key_attr {
        todo!();
        // load_struct(registry, strct, key.0, table);
    }
}

fn load_struct(registry: &AppTypeRegistry, strct: &mut dyn Struct, table: &toml::Table) {
    for i in 0..strct.field_len() {
        let key = strct.name_at(i).unwrap().to_owned();
        let field_mut = strct.field_at_mut(i).unwrap();
        match field_mut.get_represented_type_info().unwrap() {
            TypeInfo::Struct(_) => todo!(),
            TypeInfo::TupleStruct(_) => todo!(),
            TypeInfo::Tuple(_) => todo!(),
            TypeInfo::List(_) => todo!(),
            TypeInfo::Array(_) => todo!(),
            TypeInfo::Map(_) => todo!(),
            TypeInfo::Enum(en) => {
                if en.type_path().starts_with("core::option::Option") {
                    if table.contains_key(&key) {
                        let VariantInfo::Tuple(variant_info) = en.variant("Some").unwrap() else {
                            panic!("Expected Tuple variant for Some");
                        };
                        let some_field = variant_info.field_at(0).unwrap();
                        let rr = registry.read();
                        let field_type = rr.get(some_field.type_id()).unwrap();
                        let mut tuple = DynamicTuple::default();
                        tuple.set_represented_type(Some(field_type.type_info()));
                        drop(rr);
                        decode_value(tuple.as_reflect_mut(), table.get(&key).unwrap());
                        let value = DynamicEnum::new("Some", DynamicVariant::Tuple(tuple));
                        field_mut.apply(value.as_reflect());
                    } else {
                        // Key not found, set to None
                        field_mut
                            .apply(DynamicEnum::new("None", DynamicVariant::Unit).as_reflect());
                    };
                } else {
                    warn!("Preferences: Unsupported enum type: {:?}", en);
                }
            }
            TypeInfo::Value(_) => {
                if let Some(value) = table.get(&key) {
                    if let Ok(value) =
                        decode_value_boxed(field_mut.get_represented_type_info().unwrap(), value)
                    {
                        field_mut.apply(value.as_reflect())
                    }
                }
            }
        }
    }
}

fn maybe_load_tuple_struct(
    registry: &AppTypeRegistry,
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
            load_tuple_struct(registry, tuple_struct, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        load_tuple_struct(registry, tuple_struct, key.0, table);
    }
}

fn load_tuple_struct(
    registry: &AppTypeRegistry,
    tuple_struct: &mut dyn TupleStruct,
    key: &'static str,
    table: &toml::Table,
) {
    if tuple_struct.field_len() == 1 {
        let field_mut = tuple_struct.field_mut(0).unwrap();
        match field_mut.get_represented_type_info().unwrap() {
            TypeInfo::Struct(_) => todo!(),
            TypeInfo::TupleStruct(_) => todo!(),
            TypeInfo::Tuple(_) => todo!(),
            TypeInfo::List(_) => todo!(),
            TypeInfo::Array(_) => todo!(),
            TypeInfo::Map(_) => todo!(),
            TypeInfo::Enum(en) => {
                if en.type_path().starts_with("core::option::Option") {
                    if table.contains_key(key) {
                        let VariantInfo::Tuple(variant_info) = en.variant("Some").unwrap() else {
                            panic!("Expected Tuple variant for Some");
                        };
                        let some_field = variant_info.field_at(0).unwrap();
                        let rr = registry.read();
                        let field_type = rr.get(some_field.type_id()).unwrap();
                        let mut tuple = DynamicTuple::default();
                        tuple.set_represented_type(Some(field_type.type_info()));
                        drop(rr);
                        decode_value(tuple.as_reflect_mut(), table.get(key).unwrap());
                        let value = DynamicEnum::new("Some", DynamicVariant::Tuple(tuple));
                        field_mut.apply(value.as_reflect());
                    } else {
                        // Key not found, set to None
                        field_mut
                            .apply(DynamicEnum::new("None", DynamicVariant::Unit).as_reflect());
                    };
                } else {
                    warn!("Preferences: Unsupported enum type: {:?}", en);
                }
            }
            TypeInfo::Value(_) => {
                if let Some(value) = table.get(key) {
                    if let Ok(value) =
                        decode_value_boxed(field_mut.get_represented_type_info().unwrap(), value)
                    {
                        field_mut.apply(value.as_reflect())
                    }
                }
            }
        }
    }
}

fn maybe_load_enum(enum_ty: &EnumInfo, enum_mut: &mut dyn Enum, table: &toml::Table) -> bool {
    let group_attr = enum_ty.custom_attributes().get::<PreferencesGroup>();
    let key_attr = enum_ty.custom_attributes().get::<PreferencesKey>();
    if let Some(group) = group_attr {
        let Some(group_value) = table.get(group.0) else {
            return false;
        };
        let Some(group) = group_value.as_table() else {
            return false;
        };

        if let Some(key) = key_attr {
            load_enum(enum_ty, enum_mut, key.0, group);
            true
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        load_enum(enum_ty, enum_mut, key.0, table);
        true
    } else {
        false
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

fn decode_value(field: &mut dyn Reflect, value: &toml::Value) {
    match field.get_represented_type_info().unwrap() {
        TypeInfo::Struct(_) => todo!("Implement struct deserialization"),
        TypeInfo::TupleStruct(_) => todo!("Implement tuplestruct deserialization"),
        TypeInfo::Tuple(_) => todo!("Implement tuple deserialization"),
        TypeInfo::List(_) => todo!("Implement list deserialization"),
        TypeInfo::Array(_) => todo!("Implement array deserialization"),
        TypeInfo::Map(_) => todo!("Implement map deserialization"),
        TypeInfo::Enum(_) => todo!("Implement enum deserialization"),
        TypeInfo::Value(val_ty) => match value {
            toml::Value::Float(float_val) => {
                if let Some(float_field) = field.downcast_mut::<f32>() {
                    float_field.apply((*float_val as f32).as_reflect());
                } else if let Some(float_field) = field.downcast_mut::<f64>() {
                    float_field.apply((*float_val).as_reflect());
                } else {
                    warn!("Preferences: Unsupported type: {:?}", val_ty);
                }
            }
            toml::Value::String(str_val) => {
                if let Some(str_field) = field.downcast_mut::<String>() {
                    str_field.apply(str_val.as_reflect());
                } else {
                    warn!("Preferences: Unsupported type: {:?}", val_ty);
                }
            }
            _ => {
                warn!("Preferences: unsupported type: {}", val_ty.type_path());
            }
        },
    };
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum DecodeTomlError {
    #[error("Unsupported type: {0}")]
    UnsupportedConversion(&'static str),
    #[error("Unsupported source type: {0}")]
    UnsupportedSource(&'static str),
}

fn decode_value_boxed(
    ty: &TypeInfo,
    value: &toml::Value,
) -> Result<Box<dyn Reflect>, DecodeTomlError> {
    match value {
        toml::Value::Float(float_val) => {
            if ty.is::<f32>() {
                Ok((*float_val as f32).clone_value())
            } else if ty.is::<f64>() {
                Ok((*float_val).clone_value())
            } else {
                warn!("Preferences: Unsupported conversion: {:?}", ty);
                Err(DecodeTomlError::UnsupportedConversion(ty.type_path()))
            }
        }

        toml::Value::Integer(int_val) => {
            if ty.is::<f32>() {
                Ok((*int_val as f32).clone_value())
            } else if ty.is::<f64>() {
                Ok((*int_val).clone_value())
            } else if ty.is::<i8>() {
                Ok(((*int_val).clamp(i8::MIN as i64, i8::MAX as i64) as i8).clone_value())
            } else if ty.is::<i16>() {
                Ok(((*int_val).clamp(i16::MIN as i64, i16::MAX as i64) as i16).clone_value())
            } else if ty.is::<i32>() {
                Ok(((*int_val).clamp(i32::MIN as i64, i32::MAX as i64) as i32).clone_value())
            } else if ty.is::<i64>() {
                Ok((*int_val).clone_value())
            } else if ty.is::<u8>() {
                Ok(((*int_val).clamp(u8::MIN as i64, u8::MAX as i64) as u8).clone_value())
            } else if ty.is::<u16>() {
                Ok(((*int_val).clamp(u16::MIN as i64, u16::MAX as i64) as u16).clone_value())
            } else if ty.is::<u32>() {
                Ok(((*int_val).clamp(u32::MIN as i64, u32::MAX as i64) as u32).clone_value())
            } else if ty.is::<u64>() {
                Ok(((*int_val).max(0) as u64).clone_value())
            } else if ty.is::<usize>() {
                Ok((*int_val as usize).clone_value())
            } else {
                warn!("Preferences: Unsupported conversion: {:?}", ty);
                Err(DecodeTomlError::UnsupportedConversion(ty.type_path()))
            }
        }

        toml::Value::String(str_val) => {
            if ty.is::<String>() {
                Ok(str_val.clone_value())
            } else {
                warn!("Preferences: Unsupported conversion: {:?}", ty);
                Err(DecodeTomlError::UnsupportedConversion(ty.type_path()))
            }
        }

        _ => {
            warn!("Preferences: unsupported source type: {}", ty.type_path());
            Err(DecodeTomlError::UnsupportedSource(ty.type_path()))
            // Box::new(() as Reflect)
        }
    }
}
