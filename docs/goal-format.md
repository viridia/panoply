# Goal format

A prototype of goal trees written as JSON, to test what a goal format must express before asset BSN
exists ([asset-bsn.md](asset-bsn.md)). Nothing here is decided; how behavior composes is open
([Exemplars.md](Exemplars.md), Composing behavior), and the same trees declared in SAGA are sketched
there (Packages written in SAGA).

The sample is faery's Ivy (`packages/game/src/behaviors/tambry.tsx`) and the work trees she uses
(`behaviors/work.ts`).

## Shape

- A file is a catalog of trees, labeled as exemplars are, named by `file#Label`.
- A node is an object holding one goal type as its key, with that goal's fields, and `subgoals`,
  its ordered children.
- Goal types are reflected Rust components, so the JSON Schema `bevy_remote` exports from the type
  registry checks node types and field names while writing.
- A formula is a typed field written as a string, as `Interactions.when` is (`Formula<bool>`).
- A subtree is used through `Use { tree, args }`. The tree declares its parameters with types, and
  `$name` puts one into a field when the tree is instantiated, once; a formula is evaluated as the
  world changes.
- An actor would name its trees in one aspect, `Goals { trees }`, as `Dialogues` names dialogue sets
  (D86).

## Ivy

`behaviors/tambry.goals.json`:

```jsonc
{
  "Ivy": [
    // Her dialogue sets come from her Dialogues aspect (D86); Converse names none.
    { "Converse": { "when": "self.targeted_by_player", "priority": "Social" } },
    { "Sleep": { "when": "hour_between(11, 7)", "priority": "Health", "target": "ivy.bed" } },
    {
      "Sequence": { "loop": true },
      "subgoals": [
        {
          "Random": { "norepeat": true },
          "subgoals": [
            { "Use": { "tree": "work.goals.json#UseCookPot",
                       "args": { "fixture": "inn.cookpot", "mark": "use", "distance": 0.4 } } },
            { "Use": { "tree": "work.goals.json#UseCuttingBoard",
                       "args": { "fixture": "inn.worktable.0", "mark": "use", "distance": 0.6 } } },
            { "Use": { "tree": "work.goals.json#UseCuttingBoard",
                       "args": { "fixture": "inn.worktable.1", "mark": "use", "distance": 0.6 } } },
            { "Use": { "tree": "work.goals.json#UseBroom", "args": { "area": "ivy.sweep" } } },
            {
              "Contingent": {},
              "subgoals": [
                { "WalkTo": { "target": "ivy.counter" } },
                { "Wait": { "delay": 10 } }
              ]
            }
          ]
        },
        { "Wait": { "delay": 2 } }
      ]
    }
  ]
}
```

`behaviors/work.goals.json`:

```jsonc
{
  "UseCookPot": {
    "params": { "fixture": "FixtureRef", "mark": "String", "distance": "f32" },
    "root": {
      "SceneryInteraction": { "target": "$fixture", "mark": "$mark", "distance": "$distance" },
      "subgoals": [
        {
          "Contingent": {},
          "subgoals": [
            // `interaction` is what SceneryInteraction publishes to its subgoals.
            { "LookAt": { "target": "interaction.position" } },
            { "Equip": { "item": "exemplars/weapons.exem.json#StirringSpoon", "instant": true } },
            { "Pose": { "delay": 4, "clips": ["KitchenStir", "KitchenWork", "Stand"], "repeat": true } },
            { "Unequip": { "instant": true } }
          ]
        }
      ]
    }
  }
}
```

Her exemplar would hold `"Goals": { "trees": ["behaviors/tambry.goals.json#Ivy"] }`.

## Open

- **Parameters.** Arguments are checked as the tree loads, against its declared parameters, with
  errors reported against its path; the schema cannot check them, not knowing the tree a `Use`
  names. If parameters were formula variables instead, every field that might take one would be a
  `Formula<T>`, and a literal string in it would need SAGA's quotes.
- **Scoped values.** faery's `children: params => ...` hands the fixture mark's position to the
  subgoals. A formula in a subtree must read what its ancestors publish: which values a node
  publishes, how they are named, and how they relate to parameters. A node's memo cells live on its
  entity (X17).
- **Fixture references** by local id (`ivy.bed`, `inn.cookpot`), resolved in the actor's container,
  as faery's `withLocalFixture` did: PAN79's naming.
- **Collapsing.** `UseCookPot`, `UseCuttingBoard`, `UseForge` and `UseAnvil` differ only in the
  tool and the clips, so with parameters they are one tree.
- **Length.** About twice faery's TSX, mostly the nesting of `subgoals`.
