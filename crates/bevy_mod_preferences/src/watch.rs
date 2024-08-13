use bevy::{ecs::component::Tick, prelude::*, reflect::TypeInfo};

use crate::{PreferencesChanged, PreferencesGroup, PreferencesKey, PreferencesSaveTick};

/// Watches for changes in preferences and sets the `PreferencesChanged` resource to true if any.
pub fn watch_prefs_changes(world: &mut World) {
    let this_run = world.change_tick();
    let mut save_tick = world.get_resource_mut::<PreferencesSaveTick>().unwrap();
    let last_run = save_tick.0;
    save_tick.0 = this_run;
    if is_changed_prefs(world, last_run, this_run) {
        world.get_resource_mut::<PreferencesChanged>().unwrap().0 = true;
    }
}

/// Returns true if any preference item changed since the given tick.
pub fn is_changed_prefs(world: &World, last_run: Tick, this_run: Tick) -> bool {
    let registry = world.get_resource::<AppTypeRegistry>().unwrap().clone();
    for (res, _) in world.iter_resources() {
        if let Some(tid) = res.type_id() {
            if let Some(treg) = registry.read().get(tid) {
                let is_changed = world
                    .get_resource_change_ticks_by_id(res.id())
                    .map(|c| c.is_changed(last_run, this_run))
                    .unwrap_or(false);
                if !is_changed {
                    continue;
                }
                match treg.type_info() {
                    TypeInfo::Struct(stty) => {
                        let group_attr = stty.custom_attributes().get::<PreferencesGroup>();
                        let key_attr = stty.custom_attributes().get::<PreferencesKey>();
                        if group_attr.is_some() || key_attr.is_some() {
                            return true;
                        }
                    }

                    TypeInfo::TupleStruct(tsty) => {
                        let group_attr = tsty.custom_attributes().get::<PreferencesGroup>();
                        let key_attr = tsty.custom_attributes().get::<PreferencesKey>();
                        if group_attr.is_some() || key_attr.is_some() {
                            return true;
                        } else if tsty
                            .type_path()
                            .starts_with("bevy_state::state::resources::State<")
                        {
                            let state_field = tsty.field_at(0).unwrap();
                            let rr = registry.read();
                            let Some(state_type) = rr.get(state_field.type_id()) else {
                                continue;
                            };
                            match state_type.type_info() {
                                TypeInfo::Struct(ts) => {
                                    let group_attr =
                                        ts.custom_attributes().get::<PreferencesGroup>();
                                    let key_attr = ts.custom_attributes().get::<PreferencesKey>();
                                    if (group_attr.is_some() || key_attr.is_some()) && is_changed {
                                        return true;
                                    }
                                }

                                TypeInfo::TupleStruct(ts) => {
                                    let group_attr =
                                        ts.custom_attributes().get::<PreferencesGroup>();
                                    let key_attr = ts.custom_attributes().get::<PreferencesKey>();
                                    if (group_attr.is_some() || key_attr.is_some()) && is_changed {
                                        return true;
                                    }
                                }

                                TypeInfo::Enum(ety) => {
                                    let group_attr =
                                        ety.custom_attributes().get::<PreferencesGroup>();
                                    let key_attr = ety.custom_attributes().get::<PreferencesKey>();
                                    if group_attr.is_some() || key_attr.is_some() {
                                        return true;
                                    }
                                }

                                // Other types cannot be preferences since they don't have attributes.
                                _ => {}
                            }
                        }
                    }

                    TypeInfo::Enum(ety) => {
                        let group_attr = ety.custom_attributes().get::<PreferencesGroup>();
                        let key_attr = ety.custom_attributes().get::<PreferencesKey>();
                        if group_attr.is_some() || key_attr.is_some() {
                            return true;
                        }
                    }

                    // Other types cannot be preferences since they don't have attributes.
                    _ => {}
                }
            }
        }
    }
    false
}
