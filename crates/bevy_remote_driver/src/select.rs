//! `driver.select`: the entities a path of names matches (DD3.1).

use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    name::Name,
    system::{In, Query, SystemParam},
};
use bevy_remote::{BrpError, BrpResult, builtin_methods::parse_some, error_codes};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const METHOD: &str = "driver.select";

#[derive(Deserialize)]
pub(crate) struct PathParams {
    pub path: Vec<String>,
}

/// One entity a path matched, with every name from the root down to it, so a report can say which
/// of several matches was which.
#[derive(Serialize, Debug, PartialEq)]
pub(crate) struct Match {
    pub entity: Entity,
    pub path: Vec<String>,
}

#[derive(SystemParam)]
pub(crate) struct Selector<'w, 's> {
    named: Query<'w, 's, (Entity, &'static Name)>,
    names: Query<'w, 's, &'static Name>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl Selector<'_, '_> {
    pub fn select(&self, path: &[String]) -> Result<Vec<Match>, BrpError> {
        let Some((last, ancestors)) = path.split_last() else {
            return Err(BrpError {
                code: error_codes::INVALID_PARAMS,
                message: "a path needs at least one name".into(),
                data: None,
            });
        };
        let mut matches = Vec::new();
        for (entity, name) in &self.named {
            if name.as_str() != last {
                continue;
            }
            // Nearest first, so matching `ancestors` from its end is a greedy subsequence test.
            let above: Vec<&str> = self
                .parents
                .iter_ancestors(entity)
                .filter_map(|ancestor| self.names.get(ancestor).ok())
                .map(Name::as_str)
                .collect();
            let mut wanted = ancestors.iter().rev().peekable();
            for name in &above {
                if wanted.peek().is_some_and(|w| w == name) {
                    wanted.next();
                }
            }
            if wanted.peek().is_none() {
                let mut path: Vec<String> = above.iter().rev().map(|n| n.to_string()).collect();
                path.push(last.clone());
                matches.push(Match { entity, path });
            }
        }
        Ok(matches)
    }

    /// The single entity a path matches, or an error listing what it matched instead.
    pub fn select_one(&self, path: &[String]) -> Result<Entity, BrpError> {
        let mut matches = self.select(path)?;
        if matches.len() == 1 {
            return Ok(matches.remove(0).entity);
        }
        Err(BrpError {
            code: error_codes::INVALID_PARAMS,
            message: format!(
                "{} matched {} entities, not one",
                path.join("/"),
                matches.len()
            ),
            data: serde_json::to_value(&matches).ok(),
        })
    }
}

pub(crate) fn process_request(In(params): In<Option<Value>>, selector: Selector) -> BrpResult {
    let PathParams { path } = parse_some(params)?;
    serde_json::to_value(selector.select(&path)?).map_err(BrpError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use serde_json::json;

    fn run(world: &mut World, path: Value) -> BrpResult {
        world
            .run_system_cached_with(process_request, Some(json!({ "path": path })))
            .unwrap()
    }

    fn paths(result: BrpResult) -> Vec<Vec<String>> {
        let matches = result.unwrap();
        let mut paths: Vec<Vec<String>> = matches
            .as_array()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_value(m["path"].clone()).unwrap())
            .collect();
        paths.sort();
        paths
    }

    // Settings (unnamed) Jump Rebind Move Rebind
    fn settings_screen(world: &mut World) {
        let settings = world.spawn(Name::new("Settings")).id();
        let row = world.spawn(ChildOf(settings)).id();
        let jump = world.spawn((Name::new("Jump"), ChildOf(row))).id();
        world.spawn((Name::new("Rebind"), ChildOf(jump)));
        let walk = world.spawn((Name::new("Move"), ChildOf(settings))).id();
        world.spawn((Name::new("Rebind"), ChildOf(walk)));
    }

    #[test]
    fn a_path_skips_unnamed_entities_and_reports_the_full_path() {
        let mut world = World::new();
        settings_screen(&mut world);

        let found = paths(run(&mut world, json!(["Settings", "Jump", "Rebind"])));
        assert_eq!(found, [["Settings", "Jump", "Rebind"]]);

        let found = paths(run(&mut world, json!(["Jump", "Rebind"])));
        assert_eq!(found, [["Settings", "Jump", "Rebind"]]);
    }

    #[test]
    fn a_name_used_twice_matches_twice() {
        let mut world = World::new();
        settings_screen(&mut world);

        let found = paths(run(&mut world, json!(["Rebind"])));
        assert_eq!(
            found,
            [
                vec!["Settings", "Jump", "Rebind"],
                vec!["Settings", "Move", "Rebind"]
            ]
        );

        let found = paths(run(&mut world, json!(["Settings", "Rebind"])));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn ancestors_must_appear_in_order() {
        let mut world = World::new();
        settings_screen(&mut world);

        assert!(paths(run(&mut world, json!(["Jump", "Settings", "Rebind"]))).is_empty());
        assert!(paths(run(&mut world, json!(["Pause", "Rebind"]))).is_empty());
    }

    #[test]
    fn an_empty_path_is_refused() {
        let mut world = World::new();
        let err = run(&mut world, json!([])).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
    }
}
