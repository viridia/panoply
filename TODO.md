# TODO

* cursors
* TODO: Wheel rotation should only work if mouse within viewport. We'll need to add a system
  to track which region we're in.

# Project hierarchy

* actors
  * Action
  * Actor
  * ActorTemplate
  * ActorArchetype
  * ActorAffix
  * goals
    * components
      * Activate
      * ApplySkill
      * Attack
      * Contingent
      * Deactivate
      * Dialogue
      * Equip
      * FaceToward
      * LookAt
      * Park
      * Pose
      * Pursue
      * Random
      * Ranked
      * Remark
      * SceneryInteraction
      * Sequence
      * TargetEnemy
      * ThreatChange
      * Travel
      * Unequip
      * Wait
      * Wander
    * GoalRoot
    * PrioritizedGoalList
  * parking
  * SkinnedModel
  * ThreatMap
* assets
  * archetypes
    * Archetype
* audio
  * AudioFx
  * AudioFxTemplate
  * AudioFxSystem
* books
* dialogue
  * CutScene
  * DialogueSet
  * RemarkSet
* items
  * Inventory
  * InventoryItem
  * InventoryItemArchetype
* nav
  * NavigationMesh
  * NavigationMeshBuilder
  * NavController
  * NavTract
  * NavRouteRequest
  * NavRouteTask
* overlays
  * DebugPhysicsOverlay
  * TargetingCircle
  * PathVisualizer
  * TranslucentLines
  * TranslucentSprites
  * TranslucentPoints
  * TranslucentMesh
* quests
  * StageId
  * Quest
  * QuestMgr
* particles
  * MissileSystem
  * ParticleEffect
  * ParticleEmitter
  * ParticleAspect
* physics
* scenery
  * Fixture
  * FixtureArchetype
  * FixtureModels
  * FixtureObstacles
  * FixturePhysics
  * Floor
  * FloorArchetype
  * FloorModels
  * FloorObstacles
  * FloorPhysics
  * PrecinctCache
  * Precinct
  * Tier
* skills
* terrain
  * ParcelCache
  * Parcel
  * ParcelShape
  * TerrainFx
* view
  * Viewpoint
  * Portals
  * Cutaways
  * Nameplates
* world
  * Biome
  * Realm
  * World

# Convert PNG to premultiplied alpha:

convert quest.png -background black -alpha Remove quest.png -compose Copy_Opacity -composite quest.png
convert artwork/export/editor/building.png -background black -alpha Remove artwork/export/editor/building.png -compose Copy_Opacity -composite assets/editor/building.png

# Overlays

* Show
* For
* Overlay
* TranslucentShape
* TranslucentLines
* TranslucentPoints
* TranslucentSprites

Overlay structure:

Node
  old reflex?
  Children
